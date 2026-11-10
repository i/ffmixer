//! TotalMix-style submix model for the RME Fireface 800.
//!
//! Pure logic, no I/O: the app feeds it hardware crosspoints
//! ([`import()`]) and asks it for crosspoints to write ([`Mixer::render`]).
//! See `docs/totalmix-fx-reference.md` for the behaviour being modelled.

mod import;
pub mod law;
mod mixer;
pub mod names;

pub use import::{ImportReport, import};
pub use mixer::{
    Crosspoints, Mixer, MixerError, Output, Send, Source, SourceBank, SourceKind, StripRef,
};

/// Channels per row (inputs, playbacks and outputs alike).
pub const CHANNELS: usize = 28;
/// Channel pairs per row.
pub const PAIRS: usize = CHANNELS / 2;
/// Stereo hardware outputs, i.e. independent submixes.
pub const OUTPUT_PAIRS: usize = PAIRS;
