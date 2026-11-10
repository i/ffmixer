//! Minimal, read-only binding to the Linux `firewire-cdev` ABI
//! (`<linux/firewire-cdev.h>`).
//!
//! Only block **read** requests are implemented. There is no write or lock
//! transaction code in this module, so it cannot change device state.
//! Struct layouts and ioctl numbers were checked against the system header.

use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

/// ABI version we declare. Versions 6+ switch responses to a different
/// event struct; 4 keeps the classic `fw_cdev_event_response`.
const CLIENT_ABI_VERSION: u32 = 4;

const IOC_GET_INFO: libc::c_ulong = 0xc028_2300;
const IOC_SEND_REQUEST: libc::c_ulong = 0x4028_2301;

const EVENT_BUS_RESET: u32 = 0x00;
const EVENT_RESPONSE: u32 = 0x01;

/// The only transaction code this module will ever send.
const TCODE_READ_BLOCK_REQUEST: u32 = 5;

const RCODE_COMPLETE: u32 = 0x00;
const RCODE_GENERATION: u32 = 0x13;

/// Largest block read we allow; well under the FF800's async payload limit.
pub const MAX_READ_LENGTH: usize = 2048;

#[repr(C)]
#[derive(Default)]
struct GetInfo {
    version: u32,
    rom_length: u32,
    rom: u64,
    bus_reset: u64,
    bus_reset_closure: u64,
    card: u32,
}

#[repr(C)]
#[derive(Default)]
struct EventBusReset {
    closure: u64,
    kind: u32,
    node_id: u32,
    local_node_id: u32,
    bm_node_id: u32,
    irm_node_id: u32,
    root_node_id: u32,
    generation: u32,
}

#[repr(C)]
struct SendRequest {
    tcode: u32,
    length: u32,
    offset: u64,
    closure: u64,
    data: u64,
    generation: u32,
}

const _: () = assert!(std::mem::size_of::<GetInfo>() == 40);
const _: () = assert!(std::mem::size_of::<EventBusReset>() == 40);
const _: () = assert!(std::mem::size_of::<SendRequest>() == 40);

/// Offsets inside `struct fw_cdev_event_response` (size 24, data at 20).
const RESPONSE_TYPE_OFFSET: usize = 8;
const RESPONSE_RCODE_OFFSET: usize = 12;
const RESPONSE_LENGTH_OFFSET: usize = 16;
const RESPONSE_DATA_OFFSET: usize = 20;
/// Offset of `generation` inside `struct fw_cdev_event_bus_reset`.
const BUS_RESET_GENERATION_OFFSET: usize = 32;

/// An open FireWire node such as `/dev/fw1`.
pub struct Node {
    file: File,
    generation: u32,
    next_closure: u64,
}

impl Node {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let mut bus_reset = EventBusReset::default();
        let mut info = GetInfo {
            version: CLIENT_ABI_VERSION,
            bus_reset: &mut bus_reset as *mut EventBusReset as u64,
            ..GetInfo::default()
        };
        // SAFETY: `info` and `bus_reset` are correctly sized `repr(C)`
        // structs that outlive the call; the kernel only writes into them.
        let result = unsafe { libc::ioctl(file.as_raw_fd(), IOC_GET_INFO, &mut info) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            file,
            generation: bus_reset.generation,
            next_closure: 1,
        })
    }

    /// Reads `length` bytes from `offset` in the node's address space.
    /// Retries once if a bus reset invalidated the request.
    pub fn read_block(
        &mut self,
        offset: u64,
        length: usize,
        timeout: Duration,
    ) -> io::Result<Vec<u8>> {
        match self.read_block_once(offset, length, timeout) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                self.read_block_once(offset, length, timeout)
            }
            other => other,
        }
    }

    fn read_block_once(
        &mut self,
        offset: u64,
        length: usize,
        timeout: Duration,
    ) -> io::Result<Vec<u8>> {
        if length == 0 || length > MAX_READ_LENGTH || !length.is_multiple_of(4) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("read length {length} must be a non-zero multiple of 4 up to {MAX_READ_LENGTH}"),
            ));
        }

        let closure = self.next_closure;
        self.next_closure += 1;

        // The kernel ignores the data buffer for read requests, but the
        // ABI still wants a valid pointer.
        let scratch = vec![0u8; length];
        let request = SendRequest {
            tcode: TCODE_READ_BLOCK_REQUEST,
            length: length as u32,
            offset,
            closure,
            data: scratch.as_ptr() as u64,
            generation: self.generation,
        };
        // SAFETY: `request` is a correctly sized `repr(C)` struct and
        // `scratch` stays alive until the call returns.
        let result = unsafe { libc::ioctl(self.file.as_raw_fd(), IOC_SEND_REQUEST, &request) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        self.wait_for_response(closure, length, timeout)
    }

    fn wait_for_response(
        &mut self,
        closure: u64,
        length: usize,
        timeout: Duration,
    ) -> io::Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        let mut event = vec![0u8; RESPONSE_DATA_OFFSET + MAX_READ_LENGTH + 64];

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() || !self.poll_readable(remaining)? {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "no response from device"));
            }

            let size = self.file.read(&mut event)?;
            if size < RESPONSE_DATA_OFFSET {
                continue;
            }
            let event_closure = u64::from_ne_bytes(event[0..8].try_into().unwrap());
            let kind = read_u32(&event, RESPONSE_TYPE_OFFSET);

            if kind == EVENT_BUS_RESET && size >= BUS_RESET_GENERATION_OFFSET + 4 {
                self.generation = read_u32(&event, BUS_RESET_GENERATION_OFFSET);
                continue;
            }
            if kind != EVENT_RESPONSE || event_closure != closure {
                continue;
            }

            let rcode = read_u32(&event, RESPONSE_RCODE_OFFSET);
            let payload_length = read_u32(&event, RESPONSE_LENGTH_OFFSET) as usize;
            return match rcode {
                RCODE_COMPLETE if payload_length == length => Ok(
                    event[RESPONSE_DATA_OFFSET..RESPONSE_DATA_OFFSET + length].to_vec(),
                ),
                RCODE_COMPLETE => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("expected {length} bytes, got {payload_length}"),
                )),
                RCODE_GENERATION => Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "bus reset during request",
                )),
                other => Err(io::Error::other(format!("transaction failed, rcode 0x{other:02x}"))),
            };
        }
    }

    fn poll_readable(&self, timeout: Duration) -> io::Result<bool> {
        let mut fd = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let millis = timeout.as_millis().min(i32::MAX as u128) as i32;
        // SAFETY: one valid `pollfd` for the duration of the call.
        let result = unsafe { libc::poll(&mut fd, 1, millis) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(result > 0)
    }
}

fn read_u32(
    buffer: &[u8],
    offset: usize,
) -> u32 {
    u32::from_ne_bytes(buffer[offset..offset + 4].try_into().unwrap())
}
