//! Closed integer motion; frames never accumulate state.
/// Twenty-four seconds at twelve frames per second. Every motion completes its cycles over this span, so lengthening it slows everything without losing smoothness.
pub const FRAME_COUNT: usize = 96 * TICK_FRAMES;
/// Frames per tick. Phases and rhythms are written in ticks, 96 to the loop, so that this one constant sets the pace of the whole city.
pub const TICK_FRAMES: usize = 3;
/// Integer triangular oscillation, exactly periodic.
pub fn wave(frame: usize, phase: usize, amplitude: i32) -> i32 {
    let position = ((frame % FRAME_COUNT + phase * TICK_FRAMES) % FRAME_COUNT) as i32;
    let half = FRAME_COUNT as i32 / 2;
    let triangle = if position < half {
        position
    } else {
        FRAME_COUNT as i32 - position
    };
    (triangle - half / 2) * amplitude / (half / 2)
}
/// Transport modes, stacked in stage order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Slow wheeled cart.
    Handcart,
    /// Road-edge rider.
    Bicycle,
    /// Private motor car.
    Car,
    /// Long public bus.
    Bus,
    /// Street-running rail vehicle.
    Tram,
    /// Elevated multi-car train.
    Train,
}
/// Modes unlocked at each growth stage.
pub fn unlocked_modes(stage: usize) -> &'static [Mode] {
    &[
        Mode::Handcart,
        Mode::Bicycle,
        Mode::Car,
        Mode::Bus,
        Mode::Tram,
        Mode::Train,
    ][..(stage + 1).min(6)]
}
/// Ground surface, shared by rendering and route validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// Building and garden ground.
    Grass,
    /// Pedestrian paving.
    Sidewalk,
    /// Roadway including embedded rails.
    Road,
}
/// World-space pose with height and facing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pose {
    /// World horizontal coordinate.
    pub x: i32,
    /// World vertical coordinate.
    pub y: i32,
    /// Height above road.
    pub z: i32,
    /// Clockwise segment, zero through three.
    pub facing: usize,
}
/// Number of vehicles, logarithmic and monotonic in monthly activity.
pub fn vehicle_count(recent_commits: u64, stage: usize) -> usize {
    if recent_commits == 0 {
        0
    } else {
        (1 + recent_commits.ilog2() as usize * 2).min((stage + 1) * 5)
    }
}
/// A traffic assignment to one district circuit and phase.
#[derive(Clone, Copy, Debug)]
pub struct Vehicle {
    /// Vehicle skin and silhouette.
    pub mode: Mode,
    /// Index in the plan's roadway circuits.
    pub route: usize,
    /// Whole-loop phase offset.
    pub phase: usize,
}
/// Spread agents across every district before filling another slot on a circuit.
pub fn traffic(plan: &crate::plan::CityPlan, stage: usize, recent: u64) -> Vec<Vehicle> {
    let count = vehicle_count(recent, stage);
    let mut longest = std::collections::BTreeMap::<usize, usize>::new();
    for (i, route) in plan
        .roads
        .iter()
        .enumerate()
        .filter(|(_, r)| r.points.len() > 2 && r.length() >= 40)
    {
        let entry = longest.entry(route.district).or_insert(i);
        if route.length() > plan.roads[*entry].length() {
            *entry = i;
        }
    }
    let routes: Vec<_> = longest.into_values().collect();
    let mut result = Vec::new();
    let mut ordered = routes.clone();
    ordered.sort_by_key(|&i| (plan.roads[i].district, i));
    // Give each district its first vehicle, then share the rest across the network.
    let mut districts = std::collections::BTreeSet::new();
    let mut spread: Vec<_> = ordered
        .iter()
        .copied()
        .filter(|&i| districts.insert(plan.roads[i].district))
        .collect();
    spread.extend(
        ordered
            .iter()
            .copied()
            .filter(|i| !spread_contains_first(&plan.roads, *i, &ordered)),
    );
    for slot in 0..4 {
        for &route in &spread {
            let capacity = (plan.roads[route].length() / 24).clamp(1, 4) as usize;
            if slot >= capacity {
                continue;
            }
            if result.len() >= count {
                return result;
            }
            let modes = unlocked_modes(stage);
            let mut mode = modes[result.len() % modes.len()];
            if mode == Mode::Train {
                mode = Mode::Car;
            }
            if mode == Mode::Tram {
                mode = Mode::Car;
            }
            if route == 0 && slot == 0 && stage >= 4 {
                mode = Mode::Tram;
            }
            if stage >= 5 && result.len() == 1 {
                mode = Mode::Train;
            }
            result.push(Vehicle {
                mode,
                route,
                phase: slot * FRAME_COUNT / capacity,
            });
        }
    }
    result
}
fn spread_contains_first(roads: &[crate::plan::Route], i: usize, ordered: &[usize]) -> bool {
    ordered
        .iter()
        .copied()
        .find(|&j| roads[j].district == roads[i].district)
        == Some(i)
}
/// Vehicle body sampled along the same circuit, including corners.
pub fn vehicle_body(plan: &crate::plan::CityPlan, vehicle: Vehicle, frame: usize) -> Vec<Pose> {
    if vehicle.mode == Mode::Train {
        let time = (frame % FRAME_COUNT) as i32;
        let half = FRAME_COUNT as i32 / 2;
        let travel = if time < half {
            time
        } else {
            FRAME_COUNT as i32 - time
        };
        let t = -22 + travel * 44 / half;
        return (0..13)
            .map(|section| {
                let t = (t + section - 6).clamp(-28, 28);
                Pose {
                    x: 56 + t,
                    y: 56 - t,
                    z: 22,
                    facing: if time < half { 0 } else { 2 },
                }
            })
            .collect();
    }
    let route = &plan.roads[vehicle.route];
    // Shared intersection dwell preserves spacing for every vehicle in a lane, including buses.
    let f = frame % FRAME_COUNT;
    let frame = f.saturating_sub(12) * FRAME_COUNT / (FRAME_COUNT - 12);
    let distance = (((frame % FRAME_COUNT + vehicle.phase) % FRAME_COUNT) as i32 * route.length())
        / FRAME_COUNT as i32;
    let length = match vehicle.mode {
        Mode::Handcart | Mode::Bicycle => 3,
        Mode::Car => 5,
        Mode::Bus => 9,
        Mode::Tram | Mode::Train => 13,
    };
    (0..length)
        .map(|section| {
            let mut pose = route.at(distance - section);
            if vehicle.mode == Mode::Train {
                pose.z = 22;
            }
            pose
        })
        .collect()
}
/// Walk a small block in a resident's own district.
pub fn person_pose(plan: &crate::plan::CityPlan, index: usize, frame: usize) -> Pose {
    let route = &plan.walks[index % plan.walks.len()];
    route.pose(frame, index * 7 * TICK_FRAMES)
}
