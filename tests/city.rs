#[path = "fixtures/support.rs"]
mod support;
use chrono::{Datelike, Duration};
use git_village::{
    development::{STAGES, derive},
    layout::{Archetype, archetype, buildings},
    mood::{Mood, select},
    motion::FRAME_COUNT,
    output::encode_gif,
    render::Renderer,
    setting::Setting,
    snapshot::Language,
    voxel::{Frame, WIDTH, project},
};
use support::synthetic;
fn delta(a: &Frame, b: &Frame) -> usize {
    a.pixels
        .iter()
        .zip(&b.pixels)
        .filter(|(a, b)| a != b)
        .count()
}
#[test]
fn append_only_and_monotonic_growth_including_zero_and_enormous_totals() {
    let mut previous = Vec::<git_village::layout::Building>::new();
    let mut previous_traits = (0, 0, 0);
    for total in [
        0,
        1,
        24,
        25,
        249,
        250,
        999,
        1000,
        4999,
        5000,
        19999,
        20000,
        100000,
        u64::MAX,
    ] {
        let snapshot = synthetic(total);
        let development = derive(&snapshot);
        assert!(
            (
                development.stage,
                development.building_count,
                development.max_height
            ) >= previous_traits
        );
        assert!(
            development.building_count >= previous_traits.1
                && development.max_height >= previous_traits.2
        );
        let current = buildings(&snapshot);
        for old in &previous {
            let new = current
                .iter()
                .find(|new| new.x == old.x && new.y == old.y)
                .unwrap_or_else(|| panic!("existing plot {},{} retained at {total}", old.x, old.y));
            assert!(new.height >= old.height);
        }
        previous = current;
        previous_traits = (
            development.stage,
            development.building_count,
            development.max_height,
        );
        Renderer::new(&snapshot, Setting::Earth).unwrap().still();
    }
    for (index, (_, threshold)) in STAGES.iter().enumerate() {
        assert_eq!(derive(&synthetic(*threshold)).stage, index);
    }
}
#[test]
fn determinism_seam_global_palette_and_maximum_traffic_festival_budget() {
    let snapshot =
        git_village::snapshot::Snapshot::from_json(include_bytes!("fixtures/festival.json"))
            .unwrap();
    assert_eq!(select(&snapshot), Mood::Festival);
    let mut stills = Vec::new();
    for setting in Setting::ALL {
        let renderer = Renderer::new(&snapshot, setting).unwrap();
        let frames: Vec<_> = (0..FRAME_COUNT).map(|i| renderer.frame(i)).collect();
        assert_eq!(renderer.frame(7), renderer.frame(7 + FRAME_COUNT));
        let seam = delta(&frames[FRAME_COUNT - 1], &frames[0]);
        let typical = frames
            .windows(2)
            .map(|f| delta(&f[0], &f[1]))
            .sum::<usize>()
            / 95;
        assert!(
            seam <= typical,
            "{} seam {seam} typical {typical}",
            setting.name()
        );
        let bytes = encode_gif(&frames).unwrap();
        if setting == Setting::Earth {
            let duplicate = Renderer::new(&snapshot, setting).unwrap();
            let duplicate_frames: Vec<_> = (0..FRAME_COUNT)
                .map(|index| duplicate.frame(index))
                .collect();
            assert_eq!(bytes, encode_gif(&duplicate_frames).unwrap());
        }
        let mut decoder = gif::DecodeOptions::new()
            .read_info(bytes.as_slice())
            .unwrap();
        assert!(decoder.global_palette().unwrap().len() <= 768);
        let mut count = 0;
        let mut duration = 0;
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            assert_eq!(frame.dispose, gif::DisposalMethod::Keep);
            assert!(frame.palette.is_none());
            duration += u32::from(frame.delay);
            count += 1;
        }
        assert_eq!(count, FRAME_COUNT);
        assert_eq!(duration, FRAME_COUNT as u32 * 100 / 12);
        println!(
            "{}: {} bytes, {} frames; seam {seam}, mean {typical}",
            setting.name(),
            bytes.len(),
            count
        );
        stills.push(frames[0].clone());
        let mut abandoned = snapshot.clone();
        abandoned.daily_commits.clear();
        let renderer = Renderer::new(&abandoned, setting).unwrap();
        let a = renderer.frame(0);
        let b = renderer.frame(24);
        let outside = a
            .pixels
            .iter()
            .zip(&b.pixels)
            .enumerate()
            .filter(|(i, (a, b))| *i / WIDTH < 55 && a != b)
            .count();
        assert!(outside > 5, "{} lacks nature motion", setting.name());
    }
    for (i, a) in stills.iter().enumerate() {
        for b in stills.iter().skip(i + 1) {
            assert!(delta(a, b) > 20000);
        }
    }
}
#[test]
fn depth_occludes_agents_behind_towers() {
    let mut frame = Frame::new([0, 0, 0]);
    frame.cuboid(30, 30, 12, 12, 40, [150, 120, 90]);
    let (x, y) = project(35, 25, 2);
    let index = y as usize * WIDTH + x as usize;
    let previous = frame.pixels[index];
    assert!(frame.depth[index] > (35 + 25) * 8 + 2);
    frame.pixel(x, y, (35 + 25) * 8 + 2, [255, 0, 255]);
    assert_eq!(frame.pixels[index], previous);
}
#[test]
fn language_mapping_weights_fallbacks_and_robustness() {
    for (names, expected) in [
        (
            vec!["Rust", "C", "C++", "Zig", "Assembly", "Fortran"],
            Archetype::Foundry,
        ),
        (
            vec!["Python", "R", "Julia", "Jupyter Notebook", "MATLAB", "TeX"],
            Archetype::Academy,
        ),
        (
            vec![
                "JavaScript",
                "TypeScript",
                "HTML",
                "CSS",
                "Vue",
                "Svelte",
                "PHP",
            ],
            Archetype::Market,
        ),
        (
            vec!["Java", "C#", "Go", "Kotlin", "Swift", "Scala", "Dart"],
            Archetype::Tower,
        ),
        (
            vec![
                "Shell",
                "Dockerfile",
                "Nix",
                "HCL",
                "Makefile",
                "PowerShell",
                "Lua",
            ],
            Archetype::Depot,
        ),
        (
            vec![
                "Haskell", "Lisp", "OCaml", "Elixir", "Erlang", "Clojure", "Ruby",
            ],
            Archetype::Temple,
        ),
    ] {
        for name in names {
            assert_eq!(archetype(name), expected);
        }
    }
    assert_eq!(archetype("unknown"), archetype("unknown"));
    let mut snapshot = synthetic(100000);
    snapshot.languages = vec![
        Language {
            arm_earned_on: Some("2022-01-01".parse().unwrap()),
            name: "Rust".into(),
            color: None,
            commits: 90000,
        },
        Language {
            arm_earned_on: Some("2022-01-01".parse().unwrap()),
            name: "Python".into(),
            color: None,
            commits: 10000,
        },
    ];
    let plots = buildings(&snapshot);
    let major = plots.iter().filter(|b| b.district == 0).count();
    assert!(major > 0);
    for b in plots.iter().filter(|b| b.district != usize::MAX) {
        assert_eq!(b.archetype, archetype(&snapshot.languages[b.district].name));
    }
    assert!(plots.iter().any(|b| b.district == 1));
    assert_eq!(plots[0].color, buildings(&snapshot)[0].color);
    snapshot.languages.clear();
    Renderer::new(&snapshot, Setting::Earth).unwrap().still();
    snapshot.login = "界".repeat(10000);
    snapshot.total_commits = u64::MAX;
    snapshot.languages = (0..50)
        .map(|i| Language {
            arm_earned_on: None,
            name: format!("Language{i}"),
            color: None,
            commits: 1,
        })
        .collect();
    Renderer::new(&snapshot, Setting::Earth).unwrap().still();
}
#[test]
fn moods_obey_order_calendar_edges_and_sparse_days() {
    let mut snapshot = synthetic(99);
    snapshot.daily_commits.clear();
    assert_eq!(select(&snapshot), Mood::Abandoned);
    snapshot
        .daily_commits
        .insert(snapshot.observed_on - Duration::days(59), 1);
    assert_eq!(select(&snapshot), Mood::Night);
    snapshot.daily_commits.clear();
    snapshot
        .daily_commits
        .insert(snapshot.observed_on - Duration::days(13), 1);
    assert_eq!(select(&snapshot), Mood::Workday);
    snapshot.daily_commits.insert(snapshot.observed_on, 1);
    assert_eq!(select(&snapshot), Mood::MarketDay);
    snapshot.total_commits = 100;
    assert_eq!(select(&snapshot), Mood::MarketDay);
    snapshot
        .daily_commits
        .insert(snapshot.observed_on - Duration::days(1), 2);
    assert_eq!(select(&snapshot), Mood::Festival);
    snapshot.observed_on = git_village::snapshot::monday(snapshot.observed_on);
    snapshot.daily_commits.clear();
    snapshot.daily_commits.insert(snapshot.observed_on, 1);
    assert_eq!(select(&snapshot), Mood::MarketDay);
    snapshot.observed_on += Duration::days(6);
    assert_eq!(snapshot.observed_on.weekday(), chrono::Weekday::Sun);
    snapshot.total_commits = 1234;
    snapshot.daily_commits.clear();
    for offset in 1..=12 {
        snapshot
            .daily_commits
            .insert(snapshot.observed_on - Duration::days(offset * 7), 1);
    }
    snapshot
        .daily_commits
        .insert(snapshot.observed_on - Duration::days(1), 20);
    snapshot.daily_commits.insert(snapshot.observed_on, 20);
    assert_eq!(select(&snapshot), Mood::Festival);
}
#[test]
fn every_mood_keeps_nature_alive_outside_buildings() {
    for mood in [
        Mood::Abandoned,
        Mood::Night,
        Mood::MarketDay,
        Mood::Festival,
        Mood::Workday,
    ] {
        let mut snapshot = synthetic(5500);
        snapshot.daily_commits.clear();
        match mood {
            Mood::Abandoned => {}
            Mood::Night => {
                snapshot
                    .daily_commits
                    .insert(snapshot.observed_on - Duration::days(14), 1);
            }
            Mood::MarketDay => {
                snapshot.daily_commits.insert(snapshot.observed_on, 1);
            }
            Mood::Festival => {
                snapshot.total_commits = 5000;
                snapshot.languages.clear();
                snapshot.daily_commits.insert(snapshot.observed_on, 2);
                snapshot
                    .daily_commits
                    .insert(snapshot.observed_on - Duration::days(1), 2);
            }
            Mood::Workday => {
                snapshot.daily_commits.insert(snapshot.observed_on, 2);
                snapshot
                    .daily_commits
                    .insert(snapshot.observed_on - Duration::days(1), 2);
            }
        }
        assert_eq!(select(&snapshot), mood);
        let renderer = Renderer::new(&snapshot, Setting::Earth).unwrap();
        let a = renderer.frame(0);
        let b = renderer.frame(24);
        assert!(
            a.pixels[..WIDTH * 40]
                .iter()
                .zip(&b.pixels[..WIDTH * 40])
                .any(|(a, b)| a != b)
        );
        assert_eq!(renderer.frame(24), renderer.frame(24 + FRAME_COUNT));
    }
}
#[test]
fn milestones_expire_after_seven_calendar_days_and_order_wins() {
    let mut snapshot = synthetic(100);
    snapshot.daily_commits.clear();
    snapshot
        .daily_commits
        .insert(snapshot.observed_on - Duration::days(6), 1);
    assert_eq!(select(&snapshot), Mood::Festival);
    snapshot.daily_commits.clear();
    snapshot
        .daily_commits
        .insert(snapshot.observed_on - Duration::days(7), 1);
    assert_eq!(select(&snapshot), Mood::Workday);
    snapshot.daily_commits.clear();
    snapshot
        .daily_commits
        .insert(snapshot.observed_on - Duration::days(60), 1);
    assert_eq!(select(&snapshot), Mood::Abandoned);
    snapshot.daily_commits.clear();
    snapshot
        .daily_commits
        .insert(snapshot.observed_on - Duration::days(14), 1);
    assert_eq!(select(&snapshot), Mood::Night);
    snapshot.observed_on += Duration::days(1);
    snapshot.total_commits = 1234;
    snapshot.daily_commits.clear();
    for offset in 1..=12 {
        snapshot
            .daily_commits
            .insert(snapshot.observed_on - Duration::days(offset * 7), 1);
    }
    snapshot.daily_commits.insert(snapshot.observed_on, 100);
    assert_eq!(select(&snapshot), Mood::MarketDay); // Sunday is also the first active day.
}
#[test]
fn upkeep_fades_but_the_record_monument_never_shrinks() {
    let mut snapshot = synthetic(1000);
    snapshot.daily_commits.clear();
    let start = snapshot.observed_on;
    for offset in 0..10 {
        snapshot
            .daily_commits
            .insert(start - Duration::days(offset * 7), 1);
    }
    let active = derive(&snapshot);
    assert_eq!(active.longest_streak, 10);
    assert_eq!(active.current_streak, 10);
    assert_eq!(active.upkeep, 7);
    snapshot.observed_on += Duration::days(20);
    let fading = derive(&snapshot);
    assert_eq!(fading.current_streak, 0);
    assert!(fading.upkeep > 0 && fading.upkeep < 7);
    snapshot.observed_on += Duration::days(40);
    let ended = derive(&snapshot);
    assert_eq!(ended.upkeep, 0);
    assert_eq!(ended.longest_streak, 10);
}

#[test]
fn a_small_city_starts_with_neutral_civics_around_the_plaza() {
    let plots = buildings(&synthetic(100));
    assert!(
        plots
            .iter()
            .any(|b| b.district == usize::MAX && b.color == [154, 158, 146])
    );
}
