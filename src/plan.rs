//! Snapshot-owned terrain, a round city that fills outward block by block, surfaces and closed routes.
use crate::{
    development::derive,
    layout::{Archetype, Building, archetype, language_color, random, stable_hash},
    motion::{FRAME_COUNT, Pose, Surface},
    snapshot::{Language, Snapshot},
};
use std::collections::{BTreeMap, BTreeSet};
/// An integer world tile.
pub type Point = (i32, i32);
/// World coordinate of the city's center.
const CENTER: i32 = 56;
/// Street lines repeat this many tiles apart, and the central block sits between two of them.
const BLOCK: i32 = 24;
/// Plot origins inside a block, measured from its low street line.
const PLOT_OFFSETS: [i32; 2] = [5, 13];
/// Neutral civic buildings around the central plaza, claimed before any block opens.
const CIVIC_PLOTS: [Point; 3] = [(61, 61), (61, 47), (47, 61)];
/// The city stops growing here, at roughly 600,000 public commits, because its still ground and buildings alone would otherwise not fit the GIF budget even with almost no motion.
const MAX_BLOCKS: usize = 200;
/// A closed polyline with exact integer arc-length sampling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// Consecutive vertices, implicitly closed.
    pub points: Vec<Point>,
    /// Owning district: zero is the center, and higher numbers are blocks further out.
    pub district: usize,
}
impl Route {
    /// Total Manhattan arc length.
    pub fn length(&self) -> i32 {
        self.edges()
            .map(|(a, b)| (a.0 - b.0).abs() + (a.1 - b.1).abs())
            .sum()
    }
    fn edges(&self) -> impl Iterator<Item = (Point, Point)> + '_ {
        self.points
            .iter()
            .copied()
            .zip(self.points.iter().copied().cycle().skip(1))
            .take(self.points.len())
    }
    /// Position a signed distance along the circuit, including its tangent.
    pub fn at(&self, distance: i32) -> Pose {
        let mut distance = distance.rem_euclid(self.length());
        for (a, b) in self.edges() {
            let length = (a.0 - b.0).abs() + (a.1 - b.1).abs();
            if distance < length {
                let dx = (b.0 - a.0).signum();
                let dy = (b.1 - a.1).signum();
                return Pose {
                    x: a.0 + dx * distance,
                    y: a.1 + dy * distance,
                    z: 0,
                    facing: if dx > 0 {
                        0
                    } else if dy > 0 {
                        1
                    } else if dx < 0 {
                        2
                    } else {
                        3
                    },
                };
            }
            distance -= length;
        }
        unreachable!()
    }
    /// One whole lap in the unchanged 24-second loop.
    pub fn pose(&self, frame: usize, phase: usize) -> Pose {
        self.at(
            (((frame % FRAME_COUNT + phase) % FRAME_COUNT) as i32 * self.length())
                / FRAME_COUNT as i32,
        )
    }
    /// Rasterized circuit, with every intermediate tile.
    pub fn tiles(&self) -> BTreeSet<Point> {
        (0..self.length())
            .map(|d| {
                let p = self.at(d);
                (p.x, p.y)
            })
            .collect()
    }
}
/// Everything spatial is derived once from an observation.
#[derive(Clone, Debug)]
pub struct CityPlan {
    /// Connected land tile mask.
    pub land: BTreeSet<Point>,
    /// Actual ground materials.
    pub surfaces: BTreeMap<Point, Surface>,
    /// Claimed buildings, nearest the center first.
    pub buildings: Vec<Building>,
    /// Vehicle circuits around every opened block.
    pub roads: Vec<Route>,
    /// Pedestrian block circuits.
    pub walks: Vec<Route>,
    /// Elevated rail tiles, independent of ground surfaces.
    pub rail: BTreeSet<Point>,
    /// Still trees on open grass, gathered into groves.
    pub trees: Vec<Point>,
}
/// Low street line of block `index` along one axis.
fn street(index: i32) -> i32 {
    44 + BLOCK * index
}
/// Clockwise rectangle whose reversal keeps traffic on the right of each street.
fn loop_route(low: Point, high: Point, district: usize) -> Route {
    let (x0, y0) = low;
    let (x1, y1) = high;
    let mut points = vec![(x0, y1), (x1, y1), (x1, y0), (x0, y0)];
    points.rotate_left(3);
    Route { points, district }
}
fn paint(
    surfaces: &mut BTreeMap<Point, Surface>,
    land: &BTreeSet<Point>,
    route: &Route,
    half: i32,
    side: bool,
) {
    for (x, y) in route.tiles() {
        for dy in -half..=half {
            for dx in -half..=half {
                let p = (x + dx, y + dy);
                if land.contains(&p) {
                    let value = if side {
                        Surface::Sidewalk
                    } else {
                        Surface::Road
                    };
                    if !side || surfaces.get(&p) != Some(&Surface::Road) {
                        surfaces.insert(p, value);
                    }
                }
            }
        }
    }
}
/// Every block except the central one, in the permanent order they open: nearest first, with ties settled by the account.
fn block_order(account: u64) -> Vec<Point> {
    let reach = 40;
    let mut blocks: Vec<Point> = (-reach..=reach)
        .flat_map(|by| (-reach..=reach).map(move |bx| (bx, by)))
        .filter(|&block| block != (0, 0))
        .collect();
    blocks.sort_by_key(|&(bx, by)| {
        (
            bx * bx + by * by,
            stable_hash(&format!("block:{account}:{bx}:{by}")),
        )
    });
    blocks
}
fn corner_distance((bx, by): Point) -> f64 {
    let far = |index: i32| {
        (street(index) - CENTER)
            .abs()
            .max((street(index + 1) - CENTER).abs())
    };
    f64::from(far(bx)).hypot(f64::from(far(by)))
}
impl CityPlan {
    /// Build a deterministic plan without clocks or I/O.
    pub fn new(snapshot: &Snapshot) -> Self {
        Self::for_stage(snapshot, derive(snapshot).stage)
    }
    /// Build the same land at a specified road development stage for invariant checks.
    pub fn for_stage(snapshot: &Snapshot, stage: usize) -> Self {
        let development = derive(snapshot);
        let account = snapshot.account_id;
        let target = development.building_count;
        // A fixed candidate order means more commits only ever append plots.
        let mut plots = Vec::new();
        let mut opened = Vec::new();
        let civic = CIVIC_PLOTS.len().min(target);
        for block in block_order(account).into_iter().take(MAX_BLOCKS) {
            if civic + plots.len() >= target {
                break;
            }
            opened.push(block);
            let (bx, by) = block;
            let mut candidates: Vec<Point> = PLOT_OFFSETS
                .iter()
                .flat_map(|&oy| {
                    PLOT_OFFSETS
                        .iter()
                        .map(move |&ox| (street(bx) + ox, street(by) + oy))
                })
                .collect();
            candidates.sort_by_key(|&(x, y)| stable_hash(&format!("plot:{account}:{x}:{y}")));
            for (x, y) in candidates {
                // One lot in five stays green, so blocks are never solid walls.
                let mut state = stable_hash(&format!("green:{account}:{x}:{y}"));
                if random(&mut state) % 5 != 0 && civic + plots.len() < target {
                    plots.push((x, y));
                }
            }
        }
        let farthest = opened
            .iter()
            .map(|&block| corner_distance(block))
            .fold(0.0, f64::max);
        // Edge wobble scales with the island, and its reach is added so it can never cut into a street.
        let base = (farthest + 8.0).max(26.0);
        let wobble = 1.3 * (1.0 + base / 60.0);
        let radius = base + wobble;
        let mut land = BTreeSet::new();
        let span = radius.ceil() as i32 + 3;
        for y in CENTER - span..=CENTER + span {
            for x in CENTER - span..=CENTER + span {
                let (dx, dy) = (f64::from(x - CENTER), f64::from(y - CENTER));
                let angle = dy.atan2(dx);
                let edge =
                    0.6 * (angle * 5.0 + (account % 17) as f64).sin() + 0.4 * (angle * 9.0).cos();
                if dx.hypot(dy) <= radius + wobble * edge {
                    land.insert((x, y));
                }
            }
        }
        // The central square's ring is the first route and carries the tram from City onward.
        let mut roads = vec![loop_route((44, 44), (68, 68), 0)];
        let mut walks = Vec::new();
        let mut junctions = BTreeSet::new();
        let open: BTreeSet<Point> = opened.iter().copied().chain([(0, 0)]).collect();
        let mut medians = Vec::new();
        for &(bx, by) in &opened {
            let (x0, y0) = (street(bx), street(by));
            let (x1, y1) = (street(bx + 1), street(by + 1));
            let ring = bx.unsigned_abs().max(by.unsigned_abs()) as usize;
            // Two neighboring blocks' circuits together form a two-way street with its lanes two tiles off the line.
            roads.push(loop_route((x0 + 2, y0 + 2), (x1 - 2, y1 - 2), ring));
            walks.push(loop_route((x0 + 4, y0 + 4), (x1 - 4, y1 - 4), ring));
            junctions.extend([(x0, y0), (x1, y0), (x0, y1), (x1, y1)]);
            if open.contains(&(bx + 1, by)) {
                medians.push(((x1, y0), (x1, y1)));
            }
            if open.contains(&(bx, by + 1)) {
                medians.push(((x0, y1), (x1, y1)));
            }
        }
        if stage >= 1 {
            roads.push(Route {
                points: vec![(44, 56), (68, 56)],
                district: 0,
            });
        }
        if stage >= 2 {
            roads.push(Route {
                points: vec![(56, 44), (56, 68)],
                district: 0,
            });
        }
        let mut surfaces = BTreeMap::new();
        // Street corridors are reserved at their full width from the start, so widening a road never reaches a plot.
        let half = i32::from(stage >= 2);
        for road in &roads {
            paint(&mut surfaces, &land, road, half + 1, true);
        }
        for road in &roads {
            paint(&mut surfaces, &land, road, half, false);
        }
        for (a, b) in medians {
            let line = Route {
                points: vec![a, b],
                district: 0,
            };
            if stage >= 3 {
                paint(&mut surfaces, &land, &line, 0, false);
            } else if stage == 2 {
                paint(&mut surfaces, &land, &line, 0, true);
            }
        }
        // A junction joins the two lanes of every street that meets it.
        for (x, y) in junctions {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if land.contains(&(x + dx, y + dy)) {
                        surfaces.insert((x + dx, y + dy), Surface::Road);
                    }
                }
            }
        }
        if stage >= 4 {
            let line = Route {
                points: vec![(44, 56), (68, 56)],
                district: 0,
            };
            paint(&mut surfaces, &land, &line, 3, false);
        }
        if stage >= 5 {
            let line = Route {
                points: vec![(56, 44), (56, 68)],
                district: 0,
            };
            paint(&mut surfaces, &land, &line, 3, false);
        }
        let plaza = loop_route((48, 48), (52, 52), 0);
        walks.insert(0, plaza);
        for walk in &walks {
            for p in walk.tiles() {
                surfaces.insert(p, Surface::Sidewalk);
            }
        }
        let languages: Vec<&Language> = snapshot
            .languages
            .iter()
            .filter(|language| language.commits > 0)
            .collect();
        let mut buildings: Vec<Building> = CIVIC_PLOTS[..civic]
            .iter()
            .map(|&(x, y)| Building {
                x,
                y,
                width: 4,
                length: 4,
                height: 5,
                archetype: Archetype::Academy,
                color: [154, 158, 146],
                district: usize::MAX,
            })
            .collect();
        for (x, y) in plots {
            let mut state = stable_hash(&format!("size:{account}:{x}:{y}"));
            let size = random(&mut state);
            let width = 5 + (size % 2) as i32;
            let length = 5 + ((size >> 8) % 2) as i32;
            let distance = f64::from(x - CENTER).hypot(f64::from(y - CENTER));
            let tenant = plot_tenant(account, &languages, x, y);
            let family = tenant.map_or(Archetype::Academy, |language| archetype(&language.name));
            buildings.push(Building {
                x,
                y,
                width,
                length,
                height: building_height(
                    family,
                    stage,
                    development.max_height,
                    width,
                    distance,
                    radius,
                ),
                archetype: family,
                color: tenant.map_or([154, 158, 146], |language| {
                    language_color(&language.name, language.color.as_deref())
                }),
                district: tenant.map_or(usize::MAX, |language| {
                    snapshot
                        .languages
                        .iter()
                        .position(|other| other.name == language.name)
                        .unwrap()
                }),
            });
        }
        let mut plan = Self {
            land,
            surfaces,
            buildings,
            roads,
            walks,
            rail: (-28..=28).map(|t| (CENTER + t, CENTER - t)).collect(),
            trees: Vec::new(),
        };
        plan.trees = groves(&plan, account);
        plan
    }
    /// Material under a world point; callers use `land` to distinguish water.
    pub fn surface(&self, x: i32, y: i32) -> Surface {
        self.surfaces
            .get(&(x, y))
            .copied()
            .unwrap_or(Surface::Grass)
    }
}
/// Scatter trees over open grass in patches: dense groves, light scatter, and clearings. A tree's position depends only on the account and its tile, so a tree goes only when something is built on it.
fn groves(plan: &CityPlan, account: u64) -> Vec<Point> {
    let roll = |text: String| {
        let mut state = stable_hash(&text);
        random(&mut state) % 100
    };
    let mut occupied = BTreeSet::new();
    for b in &plan.buildings {
        for y in b.y - 3..=b.y + b.length + 3 {
            for x in b.x - 3..=b.x + b.width + 3 {
                occupied.insert((x, y));
            }
        }
    }
    let clear = |x: i32, y: i32| {
        !occupied.contains(&(x, y))
            && (-2..=2).all(|dy| {
                (-2..=2).all(|dx| {
                    plan.land.contains(&(x + dx, y + dy))
                        && plan.surface(x + dx, y + dy) == Surface::Grass
                })
            })
    };
    plan.land
        .iter()
        .copied()
        .filter(|&(x, y)| x.rem_euclid(4) == 0 && y.rem_euclid(4) == 0)
        .filter(|&(x, y)| {
            let patch = roll(format!(
                "patch:{account}:{}:{}",
                x.div_euclid(16),
                y.div_euclid(16)
            ));
            let chance = match patch {
                0..=29 => 70,
                30..=69 => 15,
                _ => 0,
            };
            roll(format!("tree:{account}:{x}:{y}")) < chance && clear(x, y)
        })
        .collect()
}
/// Choose which language occupies a plot. Each language's weight is its own building demand, so an active language shows up across the city in proportion. Weighted rendezvous hashing keeps the choice independent of input order, and a plot changes tenant only when another language's weight overtakes it.
fn plot_tenant<'a>(
    account: u64,
    languages: &[&'a Language],
    x: i32,
    y: i32,
) -> Option<&'a Language> {
    languages
        .iter()
        .map(|language| {
            let mut state = stable_hash(&format!("{account}:{}:{x}:{y}", language.name));
            let unit = (random(&mut state) >> 11) as f64 / (1_u64 << 53) as f64;
            let weight = district_count(language.commits) as f64;
            (-(1.0 - unit).ln() / weight, *language)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)))
        .map(|(_, language)| language)
}
/// Sublinear language weight, approximately one building per ten to fifteen commits at small-town scale.
pub fn district_count(commits: u64) -> usize {
    if commits == 0 {
        0
    } else {
        ((commits as f64 / 15.0).powf(0.78).round() as usize).max(1)
    }
}
fn building_height(
    family: Archetype,
    stage: usize,
    maximum: i32,
    width: i32,
    distance: f64,
    radius: f64,
) -> i32 {
    use Archetype::*;
    let variation = (distance as i32 / 6).rem_euclid(3);
    let base = match family {
        Market | Depot => 4 + variation,
        Foundry | Academy => 6 + variation,
        Temple => 5 + variation,
        Tower => {
            if stage <= 3 {
                8 + variation
            } else {
                // The skyline peaks downtown and falls away toward the coast.
                let share = ((radius - distance) / radius).max(0.27);
                ((f64::from(maximum) * share) as i32).max(12)
            }
        }
    };
    let decoration = match family {
        Foundry => 3,
        Academy => 4,
        Tower => 5,
        Temple => 7,
        _ => 0,
    };
    base.min(if stage <= 3 {
        width * 2 - decoration
    } else {
        maximum
    })
    .max(4)
}
