//! Stable append-only plots and language districts.
use crate::snapshot::{Snapshot, parse_color};
/// Six language silhouettes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Archetype {
    /// Industrial sawtooth.
    Foundry,
    /// Observatory dome.
    Academy,
    /// Awnings and shops.
    Market,
    /// Office slab.
    Tower,
    /// Broad warehouse.
    Depot,
    /// Tiered sanctuary.
    Temple,
}
/// Stable string hash.
pub fn stable_hash(text: &str) -> u64 {
    text.bytes().fold(14695981039346656037, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(1099511628211)
    })
}
/// SplitMix64 deterministic random stream.
pub fn random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}
/// Language family mapping.
pub fn archetype(name: &str) -> Archetype {
    use Archetype::*;
    match name {
        "Rust" | "C" | "C++" | "Zig" | "Assembly" | "Fortran" => Foundry,
        "Python" | "R" | "Julia" | "Jupyter Notebook" | "MATLAB" | "TeX" => Academy,
        "JavaScript" | "TypeScript" | "HTML" | "CSS" | "Vue" | "Svelte" | "PHP" => Market,
        "Java" | "C#" | "Go" | "Kotlin" | "Swift" | "Scala" | "Dart" => Tower,
        "Shell" | "Dockerfile" | "Nix" | "HCL" | "Makefile" | "PowerShell" | "Lua" => Depot,
        "Haskell" | "Lisp" | "OCaml" | "Elixir" | "Erlang" | "Clojure" | "Ruby" => Temple,
        _ => [Foundry, Academy, Market, Tower, Depot, Temple][stable_hash(name) as usize % 6],
    }
}
/// One immutable plot with a growing building.
#[derive(Clone, Debug)]
pub struct Building {
    /// World coordinate.
    pub x: i32,
    /// World coordinate.
    pub y: i32,
    /// Height in pixels.
    pub height: i32,
    /// Footprint width, with a permanent gap between neighbors.
    pub width: i32,
    /// Footprint length.
    pub length: i32,
    /// Silhouette.
    pub archetype: Archetype,
    /// Base RGB.
    pub color: [u8; 3],
    /// Language district index.
    pub district: usize,
}
/// Claim a prefix of a seed-shuffled fixed plot grid.
pub fn buildings(snapshot: &Snapshot) -> Vec<Building> {
    crate::plan::CityPlan::new(snapshot).buildings
}

/// GitHub color or a stable hashed fallback for a language without a color.
pub fn language_color(name: &str, color: Option<&str>) -> [u8; 3] {
    color.and_then(parse_color).unwrap_or_else(|| {
        let hash = stable_hash(name);
        [
            80 + (hash % 140) as u8,
            80 + ((hash >> 8) % 140) as u8,
            80 + ((hash >> 16) % 140) as u8,
        ]
    })
}
