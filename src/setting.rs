//! Earth materials, palette and vehicle skins.
/// The supported environment. Future settings can supply the same material data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    /// Grass and blue sky.
    Earth,
}
/// Material and transport colors independent of geometry.
pub struct Materials {
    /// Roof tile.
    pub roof: [u8; 3],
    /// Sidewalk paving.
    pub sidewalk: [u8; 3],
    /// Tree trunk material.
    pub trunk: [u8; 3],
    /// Tree canopy material.
    pub foliage: [u8; 3],
    /// Vehicle skins in mobility order.
    pub vehicles: [[u8; 3]; 6],
}
impl Setting {
    /// Supported environments.
    pub const ALL: [Self; 1] = [Self::Earth];
    /// Stable setting spelling.
    pub fn name(self) -> &'static str {
        "earth"
    }
    /// Sky, horizon, ground, roadway, earth edge.
    pub fn colors(self) -> [[u8; 3]; 5] {
        [
            [133, 190, 203],
            [185, 211, 184],
            [113, 157, 99],
            [73, 87, 93],
            [72, 105, 76],
        ]
    }
    /// Earth materials and distinct mode liveries.
    pub fn materials(self) -> Materials {
        Materials {
            trunk: [105, 83, 60],
            foliage: [65, 114, 86],
            roof: [169, 98, 76],
            sidewalk: [184, 181, 151],
            vehicles: [
                [161, 116, 67],
                [236, 190, 74],
                [214, 98, 79],
                [239, 184, 83],
                [98, 175, 159],
                [208, 218, 202],
            ],
        }
    }
}
