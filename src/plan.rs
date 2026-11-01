//! Snapshot-owned terrain, permanent districts, surfaces and closed routes.
use crate::{
    development::derive,
    layout::{Building, archetype, language_color, stable_hash},
    motion::{FRAME_COUNT, Pose, Surface},
    snapshot::Snapshot,
};
use std::collections::{BTreeMap, BTreeSet};
/// An integer world tile.
pub type Point = (i32, i32);
/// Fixed screen directions in preference order.
pub const DIRECTIONS: [Point; 8] = [
    (1, -1),
    (-1, 1),
    (1, 0),
    (0, 1),
    (0, -1),
    (-1, 0),
    (1, 1),
    (-1, -1),
];
/// A closed polyline with exact integer arc-length sampling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// Consecutive vertices, implicitly closed.
    pub points: Vec<Point>,
    /// Owning district: zero is the hub, slot plus one is an arm.
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
/// One permanently assigned language peninsula.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arm {
    /// Language name, independent of snapshot array order.
    pub language: String,
    /// Fixed direction index.
    pub slot: usize,
    /// This arm's own tile mask, including its attachment to the hub.
    pub land: BTreeSet<Point>,
    /// Permanent claimed plot origins.
    pub plots: Vec<Point>,
}
/// Everything spatial is derived once from an observation.
#[derive(Clone, Debug)]
pub struct CityPlan {
    /// Connected land tile mask.
    pub land: BTreeSet<Point>,
    /// Language arms in earning order.
    pub arms: Vec<Arm>,
    /// Actual ground materials.
    pub surfaces: BTreeMap<Point, Surface>,
    /// Claimed buildings.
    pub buildings: Vec<Building>,
    /// Vehicle circuits in every populated district.
    pub roads: Vec<Route>,
    /// Pedestrian block circuits.
    pub walks: Vec<Route>,
    /// Elevated rail tiles, independent of ground surfaces.
    pub rail: BTreeSet<Point>,
}
fn growth(commits: u64, low: f64, high: f64) -> f64 {
    low + (high - low) * ((commits as f64 + 1.0).ln() / 100001_f64.ln()).min(1.0)
}
fn local(slot: usize, t: i32, n: i32) -> Point {
    let (dx, dy) = DIRECTIONS[slot];
    (56 + dx * t - dy * n, 56 + dy * t + dx * n)
}
fn radius(seed: u64, x: i32, y: i32) -> f64 {
    let angle = ((y - 56) as f64).atan2((x - 56) as f64);
    0.8 * (angle * 5.0 + (seed % 17) as f64).sin() + 0.5 * (angle * 9.0).cos()
}
fn arm_mask(seed: u64, slot: usize, commits: u64) -> BTreeSet<Point> {
    // Physical world dimensions make the eight directions comparable while respecting the narrow screen axis.
    let (maximum_length, maximum_width, width_anchor) = if slot < 2 {
        (130.0, 36.0, 253.0)
    } else if slot >= 6 {
        (115.0, 36.0, 253.0)
    } else {
        (150.0, 50.0, 675.0)
    };
    let scale = ((DIRECTIONS[slot].0.pow(2) + DIRECTIONS[slot].1.pow(2)) as f64).sqrt();
    let tail = ((commits as f64 / 1000.0).max(1.0).ln() / 100_f64.ln()).min(1.0);
    let length = ((72.0 * (commits as f64 / 100.0).powf(0.4)).min(maximum_length * 0.92)
        + maximum_length * 0.08 * tail)
        / scale;
    let width = maximum_width * ((commits as f64 / width_anchor).powf(0.32)).min(1.0) / scale / 2.0;
    let (dx, dy) = DIRECTIONS[slot];
    let norm = (dx * dx + dy * dy) as f64;
    let mut land = BTreeSet::new();
    for y in -105..217 {
        for x in -105..217 {
            let t = ((x - 56) * dx + (y - 56) * dy) as f64 / norm;
            let n = (-(x - 56) * dy + (y - 56) * dx) as f64 / norm;
            // Narrow roots leave deep coves; a rounded cap closes a long street frontage. Both bounds grow independently of other districts.
            let w = width * (0.65 + 0.35 * (t / 24.0).clamp(0.0, 1.0));
            let edge = 0.45 * ((t * 0.31) + (seed % 23) as f64).sin();
            let cap = width.min(7.0);
            if t >= 0.0
                && t <= length
                && n.abs() <= w + edge
                && ((t - length + cap).max(0.0).powi(2) + (n * cap / (w + edge)).powi(2)).sqrt()
                    <= cap
            {
                land.insert((x, y));
            }
        }
    }
    land
}
fn axis_path(a: Point, b: Point) -> Vec<Point> {
    let mut p = a;
    let mut points = vec![p];
    // Bresenham-like Manhattan staircase preserves adjacency for diagonal avenues.
    let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs());
    for i in 1..=steps {
        let target = (a.0 + (b.0 - a.0) * i / steps, a.1 + (b.1 - a.1) * i / steps);
        if p.0 != target.0 {
            p.0 = target.0;
            points.push(p);
        }
        if p.1 != target.1 {
            p.1 = target.1;
            points.push(p);
        }
    }
    points
}
fn circuit(slot: Option<usize>, a: i32, b: i32, n: i32, district: usize) -> Route {
    let corners = if let Some(slot) = slot {
        vec![
            local(slot, a, -n),
            local(slot, b, -n),
            local(slot, b, n),
            local(slot, a, n),
        ]
    } else {
        vec![
            (56 - n, 56 - n),
            (56 + n, 56 - n),
            (56 + n, 56 + n),
            (56 - n, 56 + n),
        ]
    };
    let mut points = Vec::new();
    for i in 0..4 {
        let edge = axis_path(corners[i], corners[(i + 1) % 4]);
        points.extend(edge.into_iter().skip(1));
    }
    // With the world normal pointing right of forward travel, reversing puts each avenue lane on the right.
    points.reverse();
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
impl CityPlan {
    /// Build a deterministic plan without clocks or I/O.
    pub fn new(snapshot: &Snapshot) -> Self {
        Self::for_stage(snapshot, derive(snapshot).stage)
    }
    /// Build the same land at a specified road development stage for invariant checks.
    pub fn for_stage(snapshot: &Snapshot, stage: usize) -> Self {
        let mut development = derive(snapshot);
        development.stage = stage;
        let mut languages: Vec<_> = snapshot
            .languages
            .iter()
            .filter(|l| l.commits >= 25 && l.arm_earned_on.is_some())
            .collect();
        languages.sort_by_key(|l| (l.arm_earned_on, &l.name));
        languages.truncate(8);
        let mut arms: Vec<_> = languages
            .iter()
            .enumerate()
            .map(|(slot, l)| Arm {
                language: l.name.clone(),
                slot,
                land: arm_mask(snapshot.account_id, slot, l.commits),
                plots: Vec::new(),
            })
            .collect();
        let mut land = BTreeSet::new();
        let r = growth(snapshot.total_commits, 30.0, 32.0);
        for y in 10..103 {
            for x in 10..103 {
                if ((x - 56_i32).pow(2) + (y - 56_i32).pow(2)) as f64
                    <= (r + radius(snapshot.account_id, x, y)).powi(2)
                {
                    land.insert((x, y));
                }
            }
        }
        let hub_land = land.clone();
        for arm in &arms {
            land.extend(&arm.land);
        }
        let mut roads = vec![circuit(None, 0, 0, 12, 0)];
        // Permanent inner blocks and incremental avenue blocks never replace an earlier lane.
        for arm in &arms {
            let (dx, dy) = DIRECTIONS[arm.slot];
            let extent = arm
                .land
                .iter()
                .map(|&(x, y)| ((x - 56) * dx + (y - 56) * dy) / (dx * dx + dy * dy))
                .max()
                .unwrap_or(0);

            for a in (12..extent - 8).step_by(16) {
                let b = a + 16;
                if b <= extent - 5 {
                    roads.push(circuit(Some(arm.slot), a, b, 2, arm.slot + 1));
                }
            }
            roads.push(circuit(
                Some(arm.slot),
                12,
                (extent - 5).max(16),
                2,
                arm.slot + 1,
            ));
        }
        if development.stage >= 1 {
            roads.push(Route {
                points: vec![(44, 56), (68, 56)],
                district: 0,
            });
        }
        if development.stage >= 2 {
            roads.push(Route {
                points: vec![(56, 44), (56, 68)],
                district: 0,
            });
        }
        if development.stage >= 4 {
            roads.push(Route {
                points: vec![(24, 56), (88, 56)],
                district: 0,
            });
        }
        if development.stage >= 5 {
            roads.push(Route {
                points: vec![(56, 24), (56, 88)],
                district: 0,
            });
        }
        let mut surfaces = BTreeMap::new();
        let half = if development.stage < 2 {
            0
        } else if development.stage < 3 {
            1
        } else {
            2
        };
        for road in &roads {
            paint(&mut surfaces, &land, road, half + 1, true);
        }
        for road in &roads {
            paint(&mut surfaces, &land, road, half, false);
        }
        if development.stage >= 4 {
            paint(
                &mut surfaces,
                &land,
                &Route {
                    points: vec![(44, 56), (68, 56)],
                    district: 0,
                },
                3,
                false,
            );
        }
        if development.stage >= 5 {
            paint(
                &mut surfaces,
                &land,
                &Route {
                    points: vec![(56, 44), (56, 68)],
                    district: 0,
                },
                3,
                false,
            );
        }
        let mut plaza = circuit(None, 0, 0, 2, 0);
        for p in &mut plaza.points {
            p.0 -= 6;
            p.1 -= 6;
        }
        for p in plaza.tiles() {
            surfaces.insert(p, Surface::Sidewalk);
        }
        let mut walks = vec![plaza];
        if development.stage <= 1 {
            let edge = circuit(None, 0, 0, 13, 0);
            if edge
                .tiles()
                .iter()
                .all(|p| surfaces.get(p) == Some(&Surface::Sidewalk))
            {
                walks.push(edge);
            }
        }
        for arm in &arms {
            for t in (16..78).step_by(16) {
                let (dx, dy) = DIRECTIONS[arm.slot];
                let offset = if dx != 0 && dy != 0 {
                    (-dy, 0)
                } else {
                    (-dy, dx)
                };
                let mut a = local(arm.slot, t, 5);
                let mut b = local(arm.slot, t + 4, 5);
                if arm.slot >= 6 {
                    a.0 += offset.0;
                    a.1 += offset.1;
                    b.0 += offset.0;
                    b.1 += offset.1;
                }
                let corners = [
                    a,
                    b,
                    (b.0 + offset.0, b.1 + offset.1),
                    (a.0 + offset.0, a.1 + offset.1),
                ];
                let mut points = Vec::new();
                for i in 0..4 {
                    points.extend(
                        axis_path(corners[i], corners[(i + 1) % 4])
                            .into_iter()
                            .skip(1),
                    );
                }
                let walk = Route {
                    points,
                    district: arm.slot + 1,
                };
                if walk
                    .tiles()
                    .iter()
                    .all(|p| land.contains(p) && surfaces.get(p) != Some(&Surface::Road))
                {
                    walks.push(walk);
                }
            }
        }
        for walk in &walks {
            for p in walk.tiles() {
                surfaces.insert(p, Surface::Sidewalk);
            }
        }
        let mut plan = Self {
            land,
            arms: Vec::new(),
            surfaces,
            buildings: Vec::new(),
            roads,
            walks,
            rail: (-28..=28).map(|t| (56 + t, 56 - t)).collect(),
        };
        // Hub candidates are fixed before any arms exist; reserve every potential avenue, so new languages never evict a civic building.
        let mut hub_candidates: Vec<Point> = (30_i32..83)
            .step_by(2)
            .flat_map(|y| (30..83).step_by(2).map(move |x| (x, y)))
            .collect();
        hub_candidates.sort_by_key(|&(x, y)| {
            (
                ((x + 2 - 56).pow(2) + (y + 2 - 56).pow(2)),
                stable_hash(&format!("{}:{x}:{y}", snapshot.account_id)),
            )
        });
        let hub_languages: Vec<_> = snapshot
            .languages
            .iter()
            .enumerate()
            .filter(|(_, l)| !languages.iter().any(|a| a.name == l.name))
            .collect();
        let hub_total: usize = hub_languages
            .iter()
            .map(|(_, l)| district_count(l.commits))
            .sum();
        let civic_count = (development.building_count / 24).clamp(1, 4) + arms.len();
        let mut hub_tickets = Vec::new();
        let mut sorted_hub = hub_languages.clone();
        sorted_hub.sort_by_key(|(_, l)| &l.name);
        for (index, language) in sorted_hub {
            hub_tickets.extend(std::iter::repeat_n(index, district_count(language.commits)));
        }
        let mut hub_claimed = 0;
        for (x, y) in hub_candidates {
            let width = 4;
            let reserved = |px: i32, py: i32| {
                let a = (px - 56).abs();
                let b = (py - 56).abs();
                a <= 3
                    || b <= 3
                    || (a <= 15 && b <= 15 && ((a - 12).abs() <= 3 || (b - 12).abs() <= 3))
                    || DIRECTIONS.iter().any(|&(dx, dy)| {
                        let norm = dx * dx + dy * dy;
                        let t = ((px - 56) * dx + (py - 56) * dy) / norm;
                        let n = (-(px - 56) * dy + (py - 56) * dx) / norm;
                        t >= 12 && n.abs() <= 5
                    })
            };
            if (y..=y + width).any(|py| {
                (x..=x + width).any(|px| {
                    !hub_land.contains(&(px, py))
                        || (px - 56).pow(2) + (py - 56).pow(2) > 28 * 28
                        || reserved(px, py)
                        || plan.surface(px, py) != Surface::Grass
                })
            }) {
                continue;
            }
            if plan.buildings.iter().any(|b| {
                x <= b.x + b.width && x + width >= b.x && y <= b.y + b.length && y + width >= b.y
            }) {
                continue;
            }
            let district = hub_tickets.get(hub_claimed).copied().unwrap_or(usize::MAX);
            let language = snapshot.languages.get(district);
            plan.buildings.push(Building {
                x,
                y,
                width,
                length: width,
                height: 5,
                archetype: language
                    .map_or(crate::layout::Archetype::Academy, |l| archetype(&l.name)),
                color: language.map_or([154, 158, 146], |l| {
                    language_color(&l.name, l.color.as_deref())
                }),
                district,
            });
            hub_claimed += 1;
            if hub_claimed >= hub_total + civic_count {
                break;
            }
        }
        // Rows use fixed longitudinal spacing and language-seeded ties. Origins never depend on another arm or on the current stage.
        for (slot, language) in languages.iter().enumerate() {
            let target = district_count(language.commits);
            let mut claimed = 0;
            let scale = ((DIRECTIONS[slot].0.pow(2) + DIRECTIONS[slot].1.pow(2)) as f64).sqrt();
            let spacing = if scale > 1.0 { 7 } else { 8 };
            for t in (22..155).step_by(spacing) {
                let mut sides = [-1, 1];
                if stable_hash(&format!("{}:{}:{t}", snapshot.account_id, language.name)) % 2 == 0 {
                    sides.reverse();
                }
                for side in sides {
                    // Diagonal-world avenues need less physical frontage per axis-aligned footprint.
                    let n = if scale > 1.0 { side * 9 } else { side * 11 };
                    let (cx, cy) = local(slot, t, n);
                    let (x, y) = (cx - 3, cy - 3);
                    let hash = stable_hash(&format!(
                        "{}:{}:{t}:{side}",
                        snapshot.account_id, language.name
                    ));
                    let width = if scale > 1.0 {
                        6
                    } else {
                        6 + (hash % 2) as i32
                    };
                    let length = if scale > 1.0 {
                        6
                    } else {
                        6 + ((hash >> 8) % 2) as i32
                    };
                    if (y..=y + length).any(|py| {
                        (x..=x + width).any(|px| {
                            !arms[slot].land.contains(&(px, py))
                                || (px - 56).pow(2) + (py - 56).pow(2) < 34 * 34
                                || plan.surface(px, py) != Surface::Grass
                        })
                    }) {
                        continue;
                    }
                    if plan.buildings.iter().any(|b| {
                        x <= b.x + b.width
                            && x + width >= b.x
                            && y <= b.y + b.length
                            && y + length >= b.y
                    }) {
                        continue;
                    }
                    let district = snapshot
                        .languages
                        .iter()
                        .position(|l| l.name == language.name)
                        .unwrap();
                    let family = archetype(&language.name);
                    let height = building_height(
                        family,
                        development.stage,
                        development.max_height,
                        width,
                        t,
                    );
                    plan.buildings.push(Building {
                        x,
                        y,
                        width,
                        length,
                        height,
                        archetype: family,
                        color: language_color(&language.name, language.color.as_deref()),
                        district,
                    });
                    arms[slot].plots.push((x, y));
                    claimed += 1;
                    if claimed >= target {
                        break;
                    }
                }
                if claimed >= target {
                    break;
                }
            }
        }
        plan.arms = arms;
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

/// Sublinear street-frontage demand, approximately one building per ten to fifteen commits at small-town scale.
pub fn district_count(commits: u64) -> usize {
    if commits == 0 {
        0
    } else {
        ((commits as f64 / 15.0).powf(0.78).round() as usize).clamp(1, 96)
    }
}
fn building_height(
    family: crate::layout::Archetype,
    stage: usize,
    maximum: i32,
    width: i32,
    distance: i32,
) -> i32 {
    use crate::layout::Archetype::*;
    let variation = (distance / 6).rem_euclid(3);
    let base = match family {
        Market | Depot => 4 + variation,
        Foundry | Academy => 6 + variation,
        Temple => 5 + variation,
        Tower => {
            if stage <= 3 {
                8 + variation
            } else {
                (maximum * (130 - distance).max(35) / 130).max(12)
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
