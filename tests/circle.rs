#[path = "fixtures/support.rs"]
mod support;
use git_village::{
    motion::{FRAME_COUNT, Surface, person_pose, traffic, vehicle_body},
    plan::{CityPlan, Point},
    render::{Backdrop, Renderer},
    setting::Setting,
    snapshot::{Language, Snapshot},
    voxel::{HEIGHT, WIDTH},
};
use std::collections::{BTreeSet, VecDeque};
use support::synthetic;
fn account(count: usize, commits: u64) -> Snapshot {
    let mut s = synthetic(commits.saturating_mul(count.max(1) as u64));
    s.languages = (0..count)
        .map(|i| Language {
            name: format!("Language{i}"),
            commits,
            color: None,
            arm_earned_on: None,
        })
        .collect();
    s
}
fn real() -> Snapshot {
    Snapshot::from_json(include_bytes!("fixtures/real-town.json")).unwrap()
}
fn connected(mask: &BTreeSet<Point>) {
    let first = *mask.first().unwrap();
    let mut seen = BTreeSet::from([first]);
    let mut queue = VecDeque::from([first]);
    while let Some((x, y)) = queue.pop_front() {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let p = (x + dx, y + dy);
            if mask.contains(&p) && seen.insert(p) {
                queue.push_back(p);
            }
        }
    }
    assert_eq!(seen.len(), mask.len(), "disconnected network");
}
#[test]
fn land_is_round_in_every_direction() {
    for total in [0, 100, 1697, 20000, 200000] {
        let plan = CityPlan::new(&synthetic(total));
        let reach = |dx: f64, dy: f64| {
            (0..)
                .map(f64::from)
                .take_while(|t| {
                    plan.land.contains(&(
                        (56.0 + dx * t).round() as i32,
                        (56.0 + dy * t).round() as i32,
                    ))
                })
                .count() as f64
        };
        let reaches: Vec<f64> = (0..16)
            .map(|i| {
                let angle = f64::from(i) * std::f64::consts::TAU / 16.0;
                reach(angle.cos(), angle.sin())
            })
            .collect();
        let low = reaches.iter().copied().fold(f64::MAX, f64::min);
        let high = reaches.iter().copied().fold(0.0, f64::max);
        assert!(high <= low * 1.3, "{total}: {low} to {high}");
    }
}
#[test]
fn more_commits_keep_every_standing_plot_and_all_land() {
    let mut previous: Option<CityPlan> = None;
    for total in [0, 500, 1697, 5000, 20000, 100000, 300000] {
        let plan = CityPlan::new(&synthetic(total));
        connected(&plan.land);
        if let Some(old) = &previous {
            assert!(old.land.is_subset(&plan.land), "{total}");
            assert!(plan.buildings.len() > old.buildings.len(), "{total}");
            for b in &old.buildings {
                assert!(
                    plan.buildings.iter().any(|n| (n.x, n.y) == (b.x, b.y)),
                    "{total} lost {},{}",
                    b.x,
                    b.y
                );
            }
        }
        previous = Some(plan);
    }
}
#[test]
fn growth_is_slow_and_stops_where_the_budget_would_not_fit() {
    let size = |total| CityPlan::new(&synthetic(total)).buildings.len();
    assert!(size(1000) <= 10, "{}", size(1000));
    assert_eq!(git_village::development::derive(&synthetic(1000)).stage, 1);
    assert!(size(20_000) > size(5000) && size(300_000) > size(20_000));
    assert_eq!(size(10_000_000), size(u64::MAX));
    assert!(size(u64::MAX) <= 800);
}
#[test]
fn every_loop_fits_the_gif_budget_in_both_backdrops() {
    let festival = Snapshot::from_json(include_bytes!("fixtures/festival.json")).unwrap();
    let mut largest = festival.clone();
    largest.total_commits = u64::MAX;
    for s in [real(), festival, largest, synthetic(0), synthetic(5000)] {
        for backdrop in [Backdrop::None, Backdrop::Scene] {
            let renderer = Renderer::with_backdrop(&s, Setting::Earth, backdrop).unwrap();
            let (frames, gif) = git_village::output::encode_within_budget(renderer).unwrap();
            assert_eq!(frames.len(), FRAME_COUNT);
            println!("{} {backdrop:?}: {} bytes", s.total_commits, gif.len());
            assert!(
                gif.len() <= git_village::output::GIF_BUDGET,
                "{} {backdrop:?}: {} bytes",
                s.total_commits,
                gif.len()
            );
        }
    }
}
#[test]
fn the_budget_only_thins_motion_and_the_same_snapshot_lands_on_the_same_gif() {
    let s = synthetic(100_000);
    let encode = || {
        let renderer = Renderer::with_backdrop(&s, Setting::Earth, Backdrop::None).unwrap();
        git_village::output::encode_within_budget(renderer).unwrap()
    };
    let (frames, gif) = encode();
    assert_eq!(gif, encode().1);
    let plain = Renderer::with_backdrop(&s, Setting::Earth, Backdrop::None)
        .unwrap()
        .frame(0);
    assert_eq!(
        (frames[0].width, frames[0].height),
        (plain.width, plain.height)
    );
}
#[test]
fn population_and_traffic_keep_growing_with_the_city() {
    let walkers_and_cars = |total: u64| {
        let snapshot = synthetic(total);
        let plan = CityPlan::new(&snapshot);
        (
            git_village::motion::crowd(plan.buildings.len()),
            traffic(&plan, 5, snapshot.recent(30)).len(),
        )
    };
    let small = walkers_and_cars(5_000);
    let large = walkers_and_cars(300_000);
    assert!(
        large.0 > small.0 && large.1 > small.1,
        "{small:?} {large:?}"
    );
}
#[test]
fn the_canvas_grows_past_the_base_size_and_keeps_the_whole_island() {
    let small = Renderer::with_backdrop(&real(), Setting::Earth, Backdrop::None)
        .unwrap()
        .still();
    assert_eq!((small.width, small.height), (WIDTH, HEIGHT));
    let s = synthetic(u64::MAX);
    let plan = CityPlan::new(&s);
    for backdrop in [Backdrop::None, Backdrop::Scene] {
        let frame = Renderer::with_backdrop(&s, Setting::Earth, backdrop)
            .unwrap()
            .still();
        assert!(frame.width > WIDTH && frame.height > HEIGHT);
        for &(x, y) in &plan.land {
            let (sx, sy) = git_village::voxel::project(x, y, 0);
            assert!(sx >= frame.left && sx < frame.left + frame.width as i32);
            assert!(sy >= frame.top && sy < frame.top + frame.height as i32);
        }
    }
}
#[test]
fn large_cities_encode_at_their_full_size_and_loop() {
    let s = synthetic(u64::MAX);
    let renderer = Renderer::with_backdrop(&s, Setting::Earth, Backdrop::None).unwrap();
    let frames: Vec<_> = (0..FRAME_COUNT)
        .step_by(48)
        .map(|f| renderer.frame(f))
        .collect();
    let bounds = git_village::output::export_bounds(&frames);
    assert!(bounds.width > WIDTH);
    let gif = git_village::output::encode_gif(&frames).unwrap();
    let decoder = gif::DecodeOptions::new().read_info(gif.as_slice()).unwrap();
    assert_eq!(decoder.width() as usize, bounds.width * 2);
    assert_eq!(decoder.height() as usize, bounds.height * 2);
    assert_eq!(renderer.frame(7), renderer.frame(7 + FRAME_COUNT));
}
#[test]
fn buildings_stand_on_clear_grass_without_touching() {
    for total in [25, 1000, 20000, 100000] {
        let plan = CityPlan::new(&synthetic(total));
        for (i, b) in plan.buildings.iter().enumerate() {
            for y in b.y..=b.y + b.length {
                for x in b.x..=b.x + b.width {
                    assert!(plan.land.contains(&(x, y)));
                    assert_eq!(plan.surface(x, y), Surface::Grass, "{total} {b:?}");
                }
            }
            for other in plan.buildings.iter().skip(i + 1) {
                assert!(
                    b.x + b.width < other.x
                        || other.x + other.width < b.x
                        || b.y + b.length < other.y
                        || other.y + other.length < b.y
                );
            }
        }
    }
}
#[test]
fn every_stage_keeps_plots_and_a_connected_road_network() {
    let s = synthetic(30000);
    let first = CityPlan::for_stage(&s, 0);
    for stage in 0..6 {
        let plan = CityPlan::for_stage(&s, stage);
        let positions: Vec<_> = plan.buildings.iter().map(|b| (b.x, b.y)).collect();
        let expected: Vec<_> = first.buildings.iter().map(|b| (b.x, b.y)).collect();
        assert_eq!(positions, expected);
        let roads: BTreeSet<_> = plan
            .surfaces
            .iter()
            .filter_map(|(&p, &surface)| (surface == Surface::Road).then_some(p))
            .collect();
        connected(&roads);
        for b in &plan.buildings {
            assert_eq!(plan.surface(b.x, b.y), Surface::Grass);
        }
    }
}
#[test]
fn agents_stay_on_their_surfaces_in_a_large_city() {
    let plan = CityPlan::new(&synthetic(100_000));
    let vehicles = traffic(&plan, 5, u64::MAX);
    assert!(vehicles.len() > 10);
    for frame in (0..FRAME_COUNT).step_by(9) {
        for vehicle in &vehicles {
            for p in vehicle_body(&plan, *vehicle, frame) {
                assert!(
                    plan.surface(p.x, p.y) == Surface::Road || plan.rail.contains(&(p.x, p.y)),
                    "{vehicle:?} {p:?}"
                );
            }
        }
        for index in 0..plan.walks.len() {
            let p = person_pose(&plan, index, frame);
            assert_eq!(plan.surface(p.x, p.y), Surface::Sidewalk);
        }
    }
}
#[test]
fn languages_mix_across_the_city_in_proportion_to_their_commits() {
    let mut s = account(2, 5000);
    s.languages[1].commits = 500;
    let plan = CityPlan::new(&s);
    let count = |index| {
        plan.buildings
            .iter()
            .filter(|b| b.district == index)
            .count()
    };
    assert!(count(0) > count(1) * 2, "{} {}", count(0), count(1));
    assert!(count(1) > 0);
    let near = |index: usize| {
        plan.buildings
            .iter()
            .filter(|b| b.district == index)
            .any(|b| (b.x - 56).abs() < 60 && (b.y - 56).abs() < 60)
    };
    assert!(near(0) && near(1));
}
#[test]
fn a_tenant_depends_on_activity_not_on_input_order() {
    let mut s = account(4, 2000);
    let names = |s: &Snapshot| -> Vec<(i32, i32, String)> {
        CityPlan::new(s)
            .buildings
            .iter()
            .filter(|b| b.district != usize::MAX)
            .map(|b| (b.x, b.y, s.languages[b.district].name.clone()))
            .collect()
    };
    let before = names(&s);
    s.languages.reverse();
    assert_eq!(before, names(&s));
}
#[test]
fn growth_moves_tenants_only_toward_the_growing_language() {
    let mut s = account(4, 2000);
    let tenants = |s: &Snapshot| -> std::collections::BTreeMap<(i32, i32), String> {
        CityPlan::new(s)
            .buildings
            .iter()
            .filter(|b| b.district != usize::MAX)
            .map(|b| ((b.x, b.y), s.languages[b.district].name.clone()))
            .collect()
    };
    let before = tenants(&s);
    s.languages[0].commits = 4000;
    s.total_commits += 2000;
    let after = tenants(&s);
    for (plot, old) in &before {
        let new = &after[plot];
        assert!(old == new || new == "Language0", "{old} became {new}");
    }
}
#[test]
fn groves_stand_on_open_grass() {
    let plan = CityPlan::new(&real());
    assert!(plan.trees.len() >= 20, "{} trees", plan.trees.len());
    for &(x, y) in &plan.trees {
        assert!(plan.land.contains(&(x, y)));
        assert_eq!(plan.surface(x, y), Surface::Grass);
        assert!(
            !plan
                .buildings
                .iter()
                .any(|b| x >= b.x && x <= b.x + b.width && y >= b.y && y <= b.y + b.length)
        );
    }
}
