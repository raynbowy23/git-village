//! Environments: palettes, materials, vehicle skins and what grows on open ground.
/// An environment. A setting changes what the city looks like and how nature behaves, never what the data says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    /// Grass island in a sea under a soft sky.
    Earth,
    /// Floating island over a sea of cloud.
    Sky,
    /// Red regolith under a butterscotch sky and twin moons.
    Mars,
    /// Fields and timber at a warm dusk.
    Historic,
    /// Dark plaza with a neon grid under a violet sky.
    Futuristic,
    /// Sand floor in deep blue water.
    Underwater,
    /// Cavern floor in the dark.
    Underground,
}
/// What stands on open ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flora {
    /// Round trees and pines.
    Trees,
    /// Boulders.
    Rocks,
    /// Glowing pylons.
    Pylons,
    /// Swaying kelp.
    Kelp,
    /// Crystal clusters.
    Crystals,
}
/// What lights the scene's sky.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Celestial {
    /// One sun, or a moon at night.
    Sun,
    /// Two small moons.
    TwinMoons,
    /// Nothing overhead.
    Dark,
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
    /// Sky by mood: day, night, abandoned.
    pub sky: [[u8; 3]; 3],
    /// Sea or floor by mood: day, night, abandoned.
    pub water: [[u8; 3]; 3],
    /// Shoreline rim.
    pub rim: [u8; 3],
    /// Upper and lower island side.
    pub soil: [[u8; 3]; 2],
    /// Building shadow on the ground.
    pub shadow: [u8; 3],
    /// Drifting shapes in the sky.
    pub cloud: [u8; 3],
    /// Birds, bats or distant fish.
    pub flier: [u8; 3],
    /// How far vehicles float above the road.
    pub lift: i32,
    /// Open-ground growth.
    pub flora: Flora,
    /// Sky lighting.
    pub celestial: Celestial,
    /// Share, in percent, of this color mixed into every building, keeping the language's hue readable.
    pub tint: ([u8; 3], u16),
    /// Whether sailboats float offshore.
    pub boats: bool,
}
impl Setting {
    /// Supported environments.
    pub const ALL: [Self; 7] = [
        Self::Earth,
        Self::Sky,
        Self::Mars,
        Self::Historic,
        Self::Futuristic,
        Self::Underwater,
        Self::Underground,
    ];
    /// Stable setting spelling.
    pub fn name(self) -> &'static str {
        match self {
            Self::Earth => "earth",
            Self::Sky => "sky",
            Self::Mars => "mars",
            Self::Historic => "historic",
            Self::Futuristic => "futuristic",
            Self::Underwater => "underwater",
            Self::Underground => "underground",
        }
    }
    /// Find a setting by its stable spelling.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|setting| setting.name() == name)
    }
    /// Sky, horizon, ground, roadway, earth edge.
    pub fn colors(self) -> [[u8; 3]; 5] {
        match self {
            Self::Earth => [
                [133, 190, 203],
                [185, 211, 184],
                [113, 157, 99],
                [73, 87, 93],
                [72, 105, 76],
            ],
            Self::Sky => [
                [150, 205, 235],
                [226, 238, 244],
                [139, 182, 118],
                [122, 124, 136],
                [108, 104, 112],
            ],
            Self::Mars => [
                [214, 170, 118],
                [176, 112, 76],
                [176, 92, 58],
                [92, 70, 66],
                [120, 66, 48],
            ],
            Self::Historic => [
                [228, 160, 106],
                [150, 120, 92],
                [136, 142, 74],
                [132, 118, 100],
                [96, 78, 56],
            ],
            Self::Futuristic => [
                [42, 28, 84],
                [78, 48, 128],
                [30, 34, 58],
                [18, 20, 36],
                [24, 24, 48],
            ],
            Self::Underwater => [
                [38, 118, 160],
                [34, 96, 134],
                [196, 182, 134],
                [120, 116, 104],
                [120, 106, 80],
            ],
            Self::Underground => [
                [26, 24, 30],
                [44, 40, 46],
                [76, 72, 70],
                [50, 46, 46],
                [38, 34, 36],
            ],
        }
    }
    /// Materials and distinct mode liveries.
    pub fn materials(self) -> Materials {
        match self {
            Self::Earth => Materials {
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
                sky: [[139, 190, 204], [29, 42, 71], [154, 169, 173]],
                water: [[54, 121, 145], [26, 62, 83], [109, 133, 135]],
                rim: [213, 198, 150],
                soil: [[150, 116, 80], [116, 90, 66]],
                shadow: [65, 99, 80],
                cloud: [223, 235, 226],
                flier: [53, 74, 90],
                lift: 0,
                flora: Flora::Trees,
                celestial: Celestial::Sun,
                tint: ([0, 0, 0], 0),
                boats: true,
            },
            Self::Sky => Materials {
                trunk: [122, 104, 84],
                foliage: [92, 150, 110],
                roof: [188, 150, 84],
                sidewalk: [224, 222, 212],
                vehicles: [
                    [188, 150, 84],
                    [226, 196, 110],
                    [206, 112, 92],
                    [226, 214, 180],
                    [120, 180, 190],
                    [232, 232, 224],
                ],
                sky: [[150, 205, 235], [32, 44, 92], [176, 188, 198]],
                water: [[226, 238, 244], [70, 82, 120], [198, 204, 208]],
                rim: [236, 234, 222],
                soil: [[138, 130, 134], [100, 94, 104]],
                shadow: [96, 136, 96],
                cloud: [250, 252, 252],
                flier: [120, 140, 160],
                lift: 8,
                flora: Flora::Trees,
                celestial: Celestial::Sun,
                tint: ([236, 232, 220], 40),
                boats: false,
            },
            Self::Mars => Materials {
                trunk: [96, 62, 50],
                foliage: [142, 86, 64],
                roof: [196, 200, 204],
                sidewalk: [150, 118, 96],
                vehicles: [
                    [190, 190, 186],
                    [224, 170, 92],
                    [226, 226, 216],
                    [206, 136, 84],
                    [140, 170, 176],
                    [212, 214, 214],
                ],
                sky: [[214, 170, 118], [46, 34, 52], [176, 150, 128]],
                water: [[150, 84, 58], [58, 36, 40], [128, 100, 84]],
                rim: [204, 134, 92],
                soil: [[134, 74, 52], [100, 56, 44]],
                shadow: [130, 66, 46],
                cloud: [226, 186, 138],
                flier: [120, 78, 58],
                lift: 0,
                flora: Flora::Rocks,
                celestial: Celestial::TwinMoons,
                tint: ([206, 204, 200], 30),
                boats: false,
            },
            Self::Historic => Materials {
                trunk: [92, 70, 48],
                foliage: [84, 104, 56],
                roof: [120, 84, 58],
                sidewalk: [168, 154, 128],
                vehicles: [
                    [116, 84, 56],
                    [146, 110, 72],
                    [132, 92, 60],
                    [156, 124, 82],
                    [122, 96, 68],
                    [170, 150, 116],
                ],
                sky: [[228, 160, 106], [36, 32, 62], [178, 160, 140]],
                water: [[86, 112, 112], [28, 44, 58], [116, 124, 118]],
                rim: [190, 170, 122],
                soil: [[124, 96, 66], [96, 76, 54]],
                shadow: [92, 100, 56],
                cloud: [240, 206, 170],
                flier: [70, 54, 56],
                lift: 0,
                flora: Flora::Trees,
                celestial: Celestial::Sun,
                tint: ([168, 138, 100], 35),
                boats: true,
            },
            Self::Futuristic => Materials {
                trunk: [60, 66, 96],
                foliage: [70, 230, 220],
                roof: [40, 46, 76],
                sidewalk: [62, 70, 104],
                vehicles: [
                    [90, 230, 240],
                    [240, 90, 200],
                    [120, 240, 150],
                    [250, 220, 90],
                    [130, 150, 250],
                    [230, 236, 250],
                ],
                sky: [[42, 28, 84], [14, 10, 36], [58, 54, 76]],
                water: [[22, 20, 54], [10, 10, 30], [44, 44, 58]],
                rim: [90, 220, 230],
                soil: [[40, 44, 72], [26, 28, 52]],
                shadow: [18, 20, 38],
                cloud: [120, 86, 170],
                flier: [120, 220, 240],
                lift: 3,
                flora: Flora::Pylons,
                celestial: Celestial::Dark,
                tint: ([60, 90, 130], 40),
                boats: false,
            },
            Self::Underwater => Materials {
                trunk: [70, 110, 80],
                foliage: [52, 138, 104],
                roof: [214, 120, 104],
                sidewalk: [214, 202, 160],
                vehicles: [
                    [230, 190, 80],
                    [240, 150, 70],
                    [236, 214, 90],
                    [226, 110, 90],
                    [210, 222, 140],
                    [228, 230, 196],
                ],
                sky: [[38, 118, 160], [12, 40, 72], [66, 106, 120]],
                water: [[24, 84, 128], [8, 28, 56], [52, 82, 94]],
                rim: [224, 212, 168],
                soil: [[150, 134, 100], [112, 100, 80]],
                shadow: [150, 140, 104],
                cloud: [96, 172, 200],
                flier: [226, 170, 90],
                lift: 5,
                flora: Flora::Kelp,
                celestial: Celestial::Dark,
                tint: ([120, 200, 200], 25),
                boats: false,
            },
            Self::Underground => Materials {
                trunk: [90, 84, 96],
                foliage: [140, 110, 210],
                roof: [96, 90, 86],
                sidewalk: [112, 106, 100],
                vehicles: [
                    [126, 100, 72],
                    [150, 118, 80],
                    [138, 106, 74],
                    [160, 130, 90],
                    [130, 104, 78],
                    [170, 150, 118],
                ],
                sky: [[26, 24, 30], [14, 12, 18], [40, 38, 42]],
                water: [[34, 32, 38], [18, 16, 22], [50, 48, 50]],
                rim: [120, 112, 104],
                soil: [[66, 60, 60], [44, 40, 42]],
                shadow: [54, 50, 50],
                cloud: [60, 54, 62],
                flier: [120, 110, 130],
                lift: 0,
                flora: Flora::Crystals,
                celestial: Celestial::Dark,
                tint: ([120, 110, 100], 35),
                boats: false,
            },
        }
    }
    /// A building color in this setting's materials. Mixing keeps the language's hue readable.
    pub fn tinted(self, color: [u8; 3]) -> [u8; 3] {
        let (tint, share) = self.materials().tint;
        std::array::from_fn(|channel| {
            ((u16::from(color[channel]) * (100 - share) + u16::from(tint[channel]) * share) / 100)
                as u8
        })
    }
}
