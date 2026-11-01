#[path = "fixtures/support.rs"]
mod support;
use git_village::{
    motion::{FRAME_COUNT, Surface, person_pose, traffic},
    plan::{CityPlan, Point},
    render::{Backdrop, Renderer},
    setting::Setting,
    snapshot::{Language, Snapshot},
    voxel::{HEIGHT, WIDTH, project},
};
use std::collections::{BTreeSet, VecDeque};
fn account(count: usize, commits: u64) -> Snapshot {
    let mut s = support::synthetic(commits.saturating_mul(count.max(1) as u64));
    s.languages = (0..count)
        .map(|i| Language {
            name: format!("Language{i}"),
            commits,
            color: None,
            arm_earned_on: (commits >= 25).then(|| s.created_on + chrono::Duration::days(i as i64)),
        })
        .collect();
    s
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
    assert_eq!(
        seen.len(),
        mask.len(),
        "disconnected network {:?}",
        mask.difference(&seen).take(10).collect::<Vec<_>>()
    );
}
#[test]
fn earning_threshold_caps_and_hub_only() {
    assert!(CityPlan::new(&account(3, 24)).arms.is_empty());
    assert_eq!(CityPlan::new(&account(3, 25)).arms.len(), 3);
    assert_eq!(CityPlan::new(&account(9, 25)).arms.len(), 8);
    assert!(CityPlan::new(&account(0, 100)).arms.is_empty());
}
#[test]
fn slots_sizes_independence_and_input_order() {
    let mut s = account(3, 1000);
    let before = CityPlan::new(&s);
    s.languages[0].commits = 20000;
    s.total_commits += 19000;
    let after = CityPlan::new(&s);
    assert!(before.arms[0].land.is_subset(&after.arms[0].land));
    assert!(after.arms[0].land.len() > after.arms[1].land.len());
    for i in 1..3 {
        assert_eq!(before.arms[i].plots, after.arms[i].plots);
        assert_eq!(before.arms[i].land, after.arms[i].land);
    }
    s.languages.push(Language {
        name: "New".into(),
        commits: 25,
        color: None,
        arm_earned_on: Some(s.observed_on),
    });
    s.total_commits += 25;
    let added = CityPlan::new(&s);
    for (a, b) in after.arms.iter().zip(&added.arms) {
        assert_eq!(a.plots, b.plots);
        assert_eq!(a.land, b.land);
        assert_eq!(a.slot, b.slot);
    }
    s.languages.reverse();
    let reordered = CityPlan::new(&s);
    assert_eq!(added.land, reordered.land);
    assert_eq!(added.arms, reordered.arms);
}
#[test]
fn nesting_connected_land_and_roads_clear_nonoverlapping_plots() {
    let mut previous = BTreeSet::new();
    for commits in [25, 100, 1000, 10000, 100000] {
        let plan = CityPlan::new(&account(8, commits));
        assert!(previous.is_subset(&plan.land));
        connected(&plan.land);
        previous = plan.land.clone();
        let roads = plan
            .surfaces
            .iter()
            .filter_map(|(&p, &s)| (s == Surface::Road).then_some(p))
            .collect();
        connected(&roads);
        for (i, b) in plan.buildings.iter().enumerate() {
            for y in b.y..=b.y + b.length {
                for x in b.x..=b.x + b.width {
                    assert!(plan.land.contains(&(x, y)));
                    assert_eq!(plan.surface(x, y), Surface::Grass);
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
fn maximum_geometry_fits_and_every_district_has_agents() {
    let s = account(8, 100000);
    let plan = CityPlan::new(&s);
    for &(x, y) in &plan.land {
        for z in [-10, 0] {
            let (sx, sy) = project(x, y, z);
            assert!((2..WIDTH as i32 - 2).contains(&sx));
            assert!((2..234).contains(&sy));
        }
    }
    for b in &plan.buildings {
        let (sx, sy) = project(b.x, b.y, b.height + 18);
        assert!((2..WIDTH as i32 - 2).contains(&sx));
        assert!(sy >= 10);
    }
    for backdrop in [Backdrop::None, Backdrop::Scene] {
        let frame = Renderer::with_backdrop(&s, Setting::Earth, backdrop)
            .unwrap()
            .still();
        assert_eq!(frame.pixels.len(), WIDTH * HEIGHT);
    }
    let vehicles = traffic(&plan, 5, u64::MAX);
    for district in 0..9 {
        assert!(
            vehicles
                .iter()
                .any(|v| plan.roads[v.route].district == district),
            "traffic missing {district}"
        );
        assert!(
            plan.walks.iter().any(|r| r.district == district),
            "walk missing {district}"
        );
    }
    for index in 0..60 {
        for frame in 0..FRAME_COUNT {
            let p = person_pose(&plan, index, frame);
            assert_eq!(plan.surface(p.x, p.y), Surface::Sidewalk);
        }
    }
}
#[test]
fn metropolis_roads_remain_visible() {
    let s = account(8, 100000);
    let plan = CityPlan::new(&s);
    let frame = Renderer::with_backdrop(&s, Setting::Earth, Backdrop::None)
        .unwrap()
        .still();
    let roads: Vec<_> = plan
        .surfaces
        .iter()
        .filter_map(|(&(x, y), &s)| (s == Surface::Road).then_some((x, y)))
        .collect();
    let visible = roads
        .iter()
        .filter(|&&(x, y)| {
            let (sx, sy) = project(x, y, 0);
            [0, 1].into_iter().any(|dy| {
                let i = (sy + dy) as usize * WIDTH + sx as usize;
                frame.depth[i] <= (x + y) * 8 + 2
                    && [[73, 87, 93], [211, 207, 169], [160, 170, 166]].contains(&frame.pixels[i])
            })
        })
        .count();
    assert!(
        visible * 100 >= roads.len() * 30,
        "visible {visible}/{}",
        roads.len()
    );
}

#[test]
fn maximum_eight_arm_festival_budget_in_both_backdrops() {
    let mut s = account(8, 100000);
    s.daily_commits.clear();
    s.daily_commits.insert(s.observed_on, 799999);
    s.daily_commits
        .insert(s.observed_on - chrono::Duration::days(1), 1);
    assert_eq!(
        git_village::mood::select(&s),
        git_village::mood::Mood::Festival
    );
    let plan = CityPlan::new(&s);
    assert_eq!(traffic(&plan, 5, u64::MAX).len(), 30);
    for backdrop in [Backdrop::None, Backdrop::Scene] {
        let renderer = Renderer::with_backdrop(&s, Setting::Earth, backdrop).unwrap();
        let frames: Vec<_> = (0..FRAME_COUNT).map(|f| renderer.frame(f)).collect();
        let gif = git_village::output::encode_gif(&frames).unwrap();
        println!(
            "Maximum eight-arm Festival {backdrop:?}: {} bytes",
            gif.len()
        );
        assert!(gif.len() <= 4 * 1024 * 1024);
    }
}

#[test]
fn city_and_metropolis_agents_reach_the_hub_and_each_outer_district() {
    for commits in [1000, 100000] {
        let plan = CityPlan::new(&account(8, commits));
        let vehicles = traffic(&plan, if commits == 1000 { 4 } else { 5 }, u64::MAX);
        let mut walkers = BTreeSet::new();
        let mut drivers = BTreeSet::new();
        for frame in 0..FRAME_COUNT {
            for index in 0..60 {
                let p = person_pose(&plan, index, frame);
                let district = plan.walks[index % plan.walks.len()].district;
                if district == 0 || (p.x - 56).pow(2) + (p.y - 56).pow(2) > 40 * 40 {
                    walkers.insert(district);
                }
            }
            for vehicle in &vehicles {
                let p = git_village::motion::vehicle_body(&plan, *vehicle, frame)[0];
                let district = plan.roads[vehicle.route].district;
                if district == 0 || (p.x - 56).pow(2) + (p.y - 56).pow(2) > 40 * 40 {
                    drivers.insert(district);
                }
            }
        }
        assert_eq!(walkers, (0..9).collect());
        assert_eq!(drivers, (0..9).collect());
    }
}
