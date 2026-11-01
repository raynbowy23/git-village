#[path = "fixtures/support.rs"]
mod support;
use chrono::Duration;
use git_village::{
    development::derive,
    layout::buildings,
    mood::{Mood, select},
    motion::{FRAME_COUNT, Surface, traffic, vehicle_body},
    plan::CityPlan,
    render::{BURST_COLORS, Renderer, WARM_LIGHT, format_number, status_lines},
    setting::Setting,
    snapshot::Snapshot,
    voxel::{WIDTH, project},
};
use support::synthetic;
fn observation(mood: Mood) -> Snapshot {
    let mut snapshot = synthetic(5500);
    snapshot.daily_commits.clear();
    match mood {
        Mood::Abandoned => {}
        Mood::Night => {
            snapshot
                .daily_commits
                .insert(snapshot.observed_on - Duration::days(20), 1);
        }
        Mood::MarketDay => {
            snapshot.daily_commits.insert(snapshot.observed_on, 1);
        }
        Mood::Festival => {
            snapshot.total_commits = 5000;
            snapshot.languages.clear();
            snapshot.daily_commits.insert(snapshot.observed_on, 5);
            snapshot
                .daily_commits
                .insert(snapshot.observed_on - Duration::days(1), 5);
        }
        Mood::Workday => {
            snapshot.daily_commits.insert(snapshot.observed_on, 5);
            snapshot
                .daily_commits
                .insert(snapshot.observed_on - Duration::days(1), 5);
        }
    }
    assert_eq!(select(&snapshot), mood);
    snapshot
}
#[test]
fn upper_stages_have_distinct_count_height_and_built_area() {
    let metrics = |total| {
        let plots = buildings(&synthetic(total));
        let count = plots.len();
        let tallest = plots.iter().map(|plot| plot.height).max().unwrap();
        let area: i32 = plots.iter().map(|plot| plot.width * plot.length).sum();
        (count, tallest, area)
    };
    let town = metrics(1000);
    let city = metrics(5000);
    let metropolis = metrics(20000);
    println!("Town {town:?}; City {city:?}; Metropolis {metropolis:?}");
    assert!(metropolis.0 >= town.0 * 3);
    assert!(metropolis.1 >= town.1 * 2);
    assert!(metropolis.2 >= town.2 * 2);
    assert!(town.0 < city.0 && city.0 < metropolis.0);
    assert!(town.1 < city.1 && city.1 < metropolis.1);
    assert!(town.2 < city.2 && city.2 < metropolis.2);
    for total in [1000, 5000, 20000, u64::MAX] {
        for plot in buildings(&synthetic(total)) {
            assert!(project(plot.x, plot.y, plot.height + 10).1 >= 10);
            assert!(
                CityPlan::new(&synthetic(total))
                    .land
                    .contains(&(plot.x, plot.y))
            );
        }
    }
}
#[test]
fn roads_grow_strictly_and_every_mode_uses_its_stages_actual_road() {
    let mut previous = std::collections::BTreeSet::new();
    // Hold the land fixed to isolate stage-gated road growth.
    let snapshot = synthetic(100000);
    for stage in 0..6 {
        let plan = CityPlan::for_stage(&snapshot, stage);
        let roads: std::collections::BTreeSet<_> = plan
            .land
            .iter()
            .copied()
            .filter(|&(x, y)| plan.surface(x, y) == Surface::Road)
            .collect();
        assert!(
            roads.len() > previous.len(),
            "stage {stage}: {} <= {}",
            roads.len(),
            previous.len()
        );
        assert!(previous.is_subset(&roads));
        previous = roads;
        for vehicle in traffic(&plan, stage, u64::MAX) {
            for frame in 0..FRAME_COUNT {
                for body in vehicle_body(&plan, vehicle, frame) {
                    assert!(
                        plan.surface(body.x, body.y) == Surface::Road
                            || (vehicle.mode == git_village::motion::Mode::Train
                                && plan.rail.contains(&(body.x, body.y)))
                    );
                }
            }
        }
    }
}
#[test]
fn night_has_warm_windows_and_no_sun_or_multiplied_sun() {
    let renderer = Renderer::new(&observation(Mood::Night), Setting::Earth).unwrap();
    let frame = renderer.still();
    let sky = &frame.pixels[..WIDTH * 50];
    for color in [[248, 225, 161], [124, 112, 107], [99, 90, 96]] {
        assert!(!sky.contains(&color));
    }
    assert!(
        frame
            .pixels
            .iter()
            .filter(|&&color| color == WARM_LIGHT)
            .count()
            > 5
    );
    assert_ne!(
        renderer.frame(0).pixels[..WIDTH * 50],
        renderer.frame(24).pixels[..WIDTH * 50]
    );
}
fn saturation(snapshot: &Snapshot) -> f64 {
    let frame = Renderer::new(snapshot, Setting::Earth).unwrap().still();
    let mut sum = 0.0;
    let mut count = 0;
    for &(x, y) in &CityPlan::new(snapshot).land {
        let (sx, sy) = project(x, y, 0);
        let color = frame.pixels[sy as usize * WIDTH + sx as usize];
        let maximum = *color.iter().max().unwrap() as f64;
        let minimum = *color.iter().min().unwrap() as f64;
        sum += if maximum == 0.0 {
            0.0
        } else {
            (maximum - minimum) / maximum
        };
        count += 1;
    }
    sum / f64::from(count)
}
#[test]
fn abandoned_is_desaturated_and_has_no_traffic_or_warm_windows() {
    let abandoned = observation(Mood::Abandoned);
    let workday = observation(Mood::Workday);
    let quiet = saturation(&abandoned);
    let active = saturation(&workday);
    println!("Abandoned saturation {quiet:.3}; Workday {active:.3}");
    assert!(quiet < active * 0.55);
    let renderer = Renderer::new(&abandoned, Setting::Earth).unwrap();
    assert!(renderer.vehicles().is_empty());
    assert!(!renderer.still().pixels.contains(&WARM_LIGHT));
}
#[test]
fn festival_bursts_expand_fade_and_outshine_workday() {
    let festival = Renderer::new(&observation(Mood::Festival), Setting::Earth).unwrap();
    let workday = Renderer::new(&observation(Mood::Workday), Setting::Earth).unwrap();
    let count = |frame: git_village::voxel::Frame| {
        frame.pixels[..WIDTH * 65]
            .iter()
            .filter(|color| BURST_COLORS.contains(color))
            .count()
    };
    let counts: Vec<_> = (0..FRAME_COUNT)
        .map(|frame| count(festival.frame(frame)))
        .collect();
    let peak = *counts.iter().max().unwrap();
    let low = *counts.iter().min().unwrap();
    let normal = (0..FRAME_COUNT)
        .map(|frame| count(workday.frame(frame)))
        .max()
        .unwrap();
    println!("burst peak {peak}, minimum {low}, workday {normal}");
    assert!(peak > 100 && peak > normal + 80);
    assert!(peak > low + 20);
}
#[test]
fn status_formats_numbers_and_fits_even_maximum_values() {
    assert_eq!(format_number(1697), "1,697");
    let mut snapshot = synthetic(u64::MAX);
    snapshot.login = "界".repeat(1000);
    snapshot.daily_commits.clear();
    snapshot
        .daily_commits
        .insert(snapshot.observed_on, u64::MAX);
    let lines = status_lines(&snapshot);
    assert!(lines.iter().all(|line| line.chars().count() <= 62));
    assert!(lines[1].contains("18,446,744,073,709,551,615 COMMITS"));
    assert!(lines[2].contains("18,446,744,073,709,551,615 POP"));
    Renderer::new(&snapshot, Setting::Earth).unwrap().still();
    let market = status_lines(&observation(Mood::MarketDay));
    assert!(market[0].contains("MARKET DAY"));
    assert!(market[1].contains("5,500 COMMITS"));
    assert_eq!(derive(&snapshot).stage, 5);
}
#[test]
fn building_variation_keeps_neighbor_gaps_and_visible_ground_shadows() {
    let plan = CityPlan::new(&synthetic(20000));
    let plots = plan.buildings.clone();
    assert!(plots.iter().any(|plot| plot.width == 4));
    assert!(plots.iter().any(|plot| plot.width == 7));
    for (index, plot) in plots.iter().enumerate() {
        for other in plots.iter().skip(index + 1) {
            assert!(
                plot.x + plot.width < other.x
                    || other.x + other.width < plot.x
                    || plot.y + plot.length < other.y
                    || other.y + other.length < plot.y
            );
        }
        for y in plot.y..plot.y + plot.length {
            for x in plot.x..plot.x + plot.width {
                assert_eq!(plan.surface(x, y), Surface::Grass);
            }
        }
    }
    let frame = Renderer::new(&observation(Mood::Workday), Setting::Earth)
        .unwrap()
        .still();
    assert!(
        frame
            .pixels
            .iter()
            .filter(|&&color| color == [65, 99, 80])
            .count()
            > 10
    );
}
#[test]
fn night_window_lights_fade_with_upkeep() {
    let mut earlier = observation(Mood::Night);
    earlier.daily_commits.clear();
    earlier
        .daily_commits
        .insert(earlier.observed_on - Duration::days(14), 1);
    let mut later = earlier.clone();
    later.observed_on += Duration::days(40);
    assert_eq!(select(&later), Mood::Night);
    let count = |snapshot: &Snapshot| {
        Renderer::new(snapshot, Setting::Earth)
            .unwrap()
            .still()
            .pixels
            .iter()
            .filter(|&&color| color == WARM_LIGHT)
            .count()
    };
    assert!(count(&earlier) > count(&later));
}
#[test]
fn elevated_train_uses_one_straight_line_through_the_avenue() {
    let plan = CityPlan::new(&synthetic(100000));
    let vehicle = traffic(&plan, 5, u64::MAX)
        .into_iter()
        .find(|v| v.mode == git_village::motion::Mode::Train)
        .unwrap();
    for frame in 0..FRAME_COUNT {
        for body in vehicle_body(&plan, vehicle, frame) {
            assert_eq!(body.z, 22);
            assert_eq!(body.x + body.y, 112);
        }
    }
}
