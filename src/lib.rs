//! Deterministic cities derived from public GitHub snapshots.
#![warn(missing_docs)]
pub mod collect;
pub mod development;
pub mod layout;
pub mod mood;
pub mod motion;
pub mod output;
pub mod plan;
pub mod render;
pub mod setting;
pub mod snapshot;
pub mod voxel;

/// Display name, derived from the package manifest to simplify renaming.
pub const DISPLAY_NAME: &str = env!("CARGO_PKG_NAME");
/// Animated profile artifact name.
pub const GIF_FILE: &str = "village.gif";
/// Still profile artifact name.
pub const PNG_FILE: &str = "village.png";
