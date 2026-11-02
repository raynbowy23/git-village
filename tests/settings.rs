#[path = "fixtures/support.rs"]
mod support;
use git_village::{
    motion::FRAME_COUNT,
    output::{GIF_BUDGET, encode_within_budget},
    plan::CityPlan,
    render::{Backdrop, Renderer},
    setting::Setting,
    snapshot::Snapshot,
};
use support::synthetic;
fn real() -> Snapshot {
    Snapshot::from_json(include_bytes!("fixtures/real-town.json")).unwrap()
}
#[test]
fn every_setting_has_a_stable_name_and_parses_back() {
    assert_eq!(Setting::ALL.len(), 7);
    for setting in Setting::ALL {
        assert_eq!(Setting::parse(setting.name()), Some(setting));
    }
    assert_eq!(Setting::parse("venus"), None);
    assert_eq!(Setting::ALL[0], Setting::Earth);
}
#[test]
fn settings_look_different_from_each_other_in_both_backdrops() {
    let s = real();
    for backdrop in [Backdrop::None, Backdrop::Scene] {
        let stills: Vec<_> = Setting::ALL
            .iter()
            .map(|&setting| {
                Renderer::with_backdrop(&s, setting, backdrop)
                    .unwrap()
                    .still()
            })
            .collect();
        for (i, a) in stills.iter().enumerate() {
            for b in stills.iter().skip(i + 1) {
                let changed = a
                    .pixels
                    .iter()
                    .zip(&b.pixels)
                    .filter(|(a, b)| a != b)
                    .count();
                assert!(changed > 5000, "{backdrop:?}: only {changed} pixels differ");
            }
        }
    }
}
#[test]
fn a_setting_never_changes_what_the_data_says() {
    let s = synthetic(20000);
    let plan = CityPlan::new(&s);
    for setting in Setting::ALL {
        let renderer = Renderer::new(&s, setting).unwrap();
        assert_eq!(renderer.vehicles().len(), renderer.vehicles().len());
        let frame = renderer.frame(7);
        assert_eq!(frame, renderer.frame(7 + FRAME_COUNT), "{}", setting.name());
    }
    assert!(!plan.buildings.is_empty());
    let earth = Renderer::new(&s, Setting::Earth).unwrap();
    for setting in Setting::ALL {
        let other = Renderer::new(&s, setting).unwrap();
        assert_eq!(earth.vehicles().len(), other.vehicles().len());
        assert_eq!(earth.crane_count(), other.crane_count());
    }
}
#[test]
fn earth_keeps_language_colors_exact_and_others_keep_the_hue_readable() {
    let color = [200, 40, 40];
    assert_eq!(Setting::Earth.tinted(color), color);
    for setting in Setting::ALL {
        let tinted = setting.tinted(color);
        assert!(tinted[0] > tinted[1], "{}: {tinted:?}", setting.name());
    }
}
#[test]
fn every_setting_fits_the_gif_budget_at_your_size_and_the_largest() {
    let festival = Snapshot::from_json(include_bytes!("fixtures/festival.json")).unwrap();
    let mut largest = festival.clone();
    largest.total_commits = u64::MAX;
    let snapshots = [real(), largest];
    std::thread::scope(|scope| {
        for setting in Setting::ALL {
            let snapshots = &snapshots;
            scope.spawn(move || {
                for s in snapshots {
                    for backdrop in [Backdrop::None, Backdrop::Scene] {
                        let renderer = Renderer::with_backdrop(s, setting, backdrop).unwrap();
                        let (_, gif) = encode_within_budget(renderer).unwrap();
                        println!("{} {backdrop:?}: {} bytes", setting.name(), gif.len());
                        assert!(
                            gif.len() <= GIF_BUDGET,
                            "{} {backdrop:?} {}: {} bytes",
                            setting.name(),
                            s.total_commits,
                            gif.len()
                        );
                    }
                }
            });
        }
    });
}
