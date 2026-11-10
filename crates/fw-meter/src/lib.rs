//! Read-only level meters for the RME Fireface 800.
//!
//! The unit publishes its levels as a block at FireWire address
//! [`METER_OFFSET`]. The layout below comes from the reverse-engineering
//! documented in snd-firewire-ctl-services
//! (`protocols/fireface/src/former.rs`); this is an independent
//! implementation of that format.
//!
//! Block layout (little-endian):
//!
//! 1. 8 bytes per physical input, then 8 bytes per physical output
//!    (pre-fader), then 8 bytes per physical output (post-fader).
//!    Not yet understood; ignored.
//! 2. 4 bytes per physical input, then per playback stream, then per
//!    physical output. Linear level, 0 to [`LEVEL_FULL_SCALE`].

mod cdev;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use cdev::Node;

pub const METER_OFFSET: u64 = 0x0000_8010_0000;

/// 10 analog, 2 SPDIF, 16 ADAT.
pub const PHYSICAL_INPUTS: usize = 28;
pub const PLAYBACK_STREAMS: usize = 28;
/// 10 analog, 2 SPDIF, 16 ADAT.
pub const PHYSICAL_OUTPUTS: usize = 28;

pub const LEVEL_FULL_SCALE: u32 = 0x7fff_ff00;
const LEVEL_MASK: u32 = 0x7fff_ff00;

const WIDE_REGION_LENGTH: usize = 8 * (PHYSICAL_INPUTS + 2 * PHYSICAL_OUTPUTS);
const LEVEL_COUNT: usize = PHYSICAL_INPUTS + PLAYBACK_STREAMS + PHYSICAL_OUTPUTS;
pub const METER_BLOCK_LENGTH: usize = WIDE_REGION_LENGTH + 4 * LEVEL_COUNT;

const READ_TIMEOUT: Duration = Duration::from_millis(200);

/// One reading of all meters, as raw linear levels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeterFrame {
    pub inputs: [u32; PHYSICAL_INPUTS],
    pub playbacks: [u32; PLAYBACK_STREAMS],
    pub outputs: [u32; PHYSICAL_OUTPUTS],
}

impl MeterFrame {
    pub fn parse(block: &[u8]) -> io::Result<Self> {
        if block.len() != METER_BLOCK_LENGTH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "meter block is {} bytes, expected {METER_BLOCK_LENGTH}",
                    block.len()
                ),
            ));
        }
        let level = |index: usize| {
            let start = WIDE_REGION_LENGTH + 4 * index;
            u32::from_le_bytes(block[start..start + 4].try_into().unwrap()) & LEVEL_MASK
        };
        Ok(Self {
            inputs: std::array::from_fn(level),
            playbacks: std::array::from_fn(|i| level(PHYSICAL_INPUTS + i)),
            outputs: std::array::from_fn(|i| level(PHYSICAL_INPUTS + PLAYBACK_STREAMS + i)),
        })
    }
}

/// Converts a raw meter level to dBFS. Returns `None` for silence.
pub fn level_to_dbfs(level: u32) -> Option<f64> {
    if level == 0 {
        None
    } else {
        Some(20.0 * (f64::from(level) / f64::from(LEVEL_FULL_SCALE)).log10())
    }
}

/// Reads Fireface 800 meters. Owns the open `/dev/fw*` node.
pub struct Ff800Meters {
    node: Node,
}

impl Ff800Meters {
    /// Opens the FireWire node whose GUID matches, e.g. `000a35004aa780e6`.
    pub fn open_by_guid(guid: &str) -> io::Result<Self> {
        let path = find_node_by_guid(Path::new("/sys/bus/firewire/devices"), guid)?;
        Ok(Self {
            node: Node::open(&path)?,
        })
    }

    pub fn read(&mut self) -> io::Result<MeterFrame> {
        let block = self
            .node
            .read_block(METER_OFFSET, METER_BLOCK_LENGTH, READ_TIMEOUT)?;
        MeterFrame::parse(&block)
    }
}

/// Finds `/dev/fwN` for a device GUID by scanning sysfs.
pub fn find_node_by_guid(
    sysfs_devices: &Path,
    guid: &str,
) -> io::Result<PathBuf> {
    let wanted = parse_guid(guid)?;
    for entry in fs::read_dir(sysfs_devices)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        // Device nodes are `fwN`; `fwN.M` entries are units within them.
        if !name.starts_with("fw") || name.contains('.') {
            continue;
        }
        let Ok(contents) = fs::read_to_string(entry.path().join("guid")) else {
            continue;
        };
        if parse_guid(contents.trim()).ok() == Some(wanted) {
            return Ok(PathBuf::from("/dev").join(name));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("no FireWire node with GUID {guid}"),
    ))
}

fn parse_guid(text: &str) -> io::Result<u64> {
    let digits = text.trim_start_matches("0x");
    u64::from_str_radix(digits, 16).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid GUID `{text}`"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_length_matches_documented_layout() {
        assert_eq!(METER_BLOCK_LENGTH, 1008);
    }

    #[test]
    fn parses_level_region_in_order() {
        let mut block = vec![0xffu8; METER_BLOCK_LENGTH];
        for index in 0..LEVEL_COUNT {
            let start = WIDE_REGION_LENGTH + 4 * index;
            let value = (index as u32 + 1) << 8;
            block[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
        let frame = MeterFrame::parse(&block).unwrap();
        assert_eq!(frame.inputs[0], 1 << 8);
        assert_eq!(frame.playbacks[0], 29 << 8);
        assert_eq!(frame.outputs[27], 84 << 8);
    }

    #[test]
    fn masks_low_byte() {
        let mut block = vec![0u8; METER_BLOCK_LENGTH];
        block[WIDE_REGION_LENGTH..WIDE_REGION_LENGTH + 4]
            .copy_from_slice(&0x1234_56ffu32.to_le_bytes());
        assert_eq!(MeterFrame::parse(&block).unwrap().inputs[0], 0x1234_5600);
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(MeterFrame::parse(&[0u8; 16]).is_err());
    }

    #[test]
    fn full_scale_is_zero_dbfs() {
        assert_eq!(level_to_dbfs(LEVEL_FULL_SCALE), Some(0.0));
        assert_eq!(level_to_dbfs(0), None);
    }

    #[test]
    fn guid_accepts_sysfs_and_ffado_forms() {
        assert_eq!(
            parse_guid("0x000a35004aa780e6").unwrap(),
            0x000a_3500_4aa7_80e6
        );
        assert_eq!(
            parse_guid("000a35004aa780e6").unwrap(),
            0x000a_3500_4aa7_80e6
        );
    }
}
