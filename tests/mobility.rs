#[path = "fixtures/support.rs"]
mod support;
use git_village::{
    motion::{
        FRAME_COUNT, Mode, Surface, person_pose, traffic, unlocked_modes, vehicle_body,
        vehicle_count,
    },
    plan::CityPlan,
};
use support::synthetic;
#[test]
fn stages_unlock_exactly_the_cumulative_modes_and_traffic_is_monotonic() {
    let expected = [
        Mode::Handcart,
        Mode::Bicycle,
        Mode::Car,
        Mode::Bus,
        Mode::Tram,
        Mode::Train,
    ];
    for stage in 0..6 {
        assert_eq!(unlocked_modes(stage), &expected[..=stage]);
    }
    assert_eq!(vehicle_count(0), 0);
    let mut previous = 0;
    for activity in 1..10000 {
        let current = vehicle_count(activity);
        assert!(current >= previous);
        previous = current;
    }
    assert!(vehicle_count(u64::MAX) > vehicle_count(10000));
}
#[test]
fn every_agent_stays_on_its_surface_and_lane_neighbors_never_overlap() {
    for stage in 0..6 {
        let plan = CityPlan::for_stage(&synthetic(100000), stage);
        let vehicles = traffic(&plan, stage, u64::MAX);
        for frame in 0..FRAME_COUNT {
            for (i, vehicle) in vehicles.iter().enumerate() {
                let body = vehicle_body(&plan, *vehicle, frame);
                for p in &body {
                    assert!(
                        plan.surface(p.x, p.y) == Surface::Road
                            || (vehicle.mode == Mode::Train && plan.rail.contains(&(p.x, p.y))),
                        "{stage} {vehicle:?} {p:?}"
                    );
                    assert_eq!(p.z, if vehicle.mode == Mode::Train { 22 } else { 0 });
                }
                for other in vehicles
                    .iter()
                    .skip(i + 1)
                    .filter(|v| v.route == vehicle.route)
                {
                    let second = vehicle_body(&plan, *other, frame);
                    assert!(
                        !body.iter().any(|a| second
                            .iter()
                            .any(|b| a.x == b.x && a.y == b.y && a.z == b.z)),
                        "lane overlap {vehicle:?} {other:?}"
                    );
                }
            }
            for index in 0..60 {
                let p = person_pose(&plan, index, frame);
                assert_eq!(
                    plan.surface(p.x, p.y),
                    Surface::Sidewalk,
                    "{stage} person {index} {p:?}"
                );
            }
        }
    }
}
#[test]
fn motion_cycles_close_and_the_seam_is_an_ordinary_step() {
    let plan = CityPlan::new(&synthetic(100000));
    for v in traffic(&plan, 5, u64::MAX) {
        let poses: Vec<_> = (0..FRAME_COUNT)
            .map(|f| vehicle_body(&plan, v, f)[0])
            .collect();
        for frame in 0..FRAME_COUNT {
            assert_eq!(poses[frame], vehicle_body(&plan, v, frame + FRAME_COUNT)[0]);
            let next = poses[(frame + 1) % FRAME_COUNT];
            let step = (poses[frame].x - next.x).abs() + (poses[frame].y - next.y).abs();
            assert!(step <= 3, "{v:?} step {step}");
        }
    }
    for index in 0..60 {
        for frame in 0..FRAME_COUNT {
            let a = person_pose(&plan, index, frame);
            let b = person_pose(&plan, index, frame + 1);
            assert!((a.x - b.x).abs() + (a.y - b.y).abs() <= 1);
            assert_eq!(a, person_pose(&plan, index, frame + FRAME_COUNT));
        }
    }
}

#[test]
fn opposing_stretches_have_distinct_right_hand_lanes() {
    let plan = CityPlan::new(&synthetic(100000));
    for vehicle in traffic(&plan, 5, u64::MAX) {
        let route = &plan.roads[vehicle.route];
        let mut headings = std::collections::BTreeMap::new();
        for distance in 0..route.length() {
            let p = route.at(distance);
            if let Some(previous) = headings.insert((p.x, p.y), p.facing) {
                assert_eq!(previous, p.facing, "opposite headings share a lane");
            }
        }
    }
}
