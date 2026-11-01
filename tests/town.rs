use git_village::{
    plan::{CityPlan, DIRECTIONS},
    render::{bare_ground, crane_count},
    setting::Setting,
    snapshot::Snapshot,
    voxel::{HEIGHT, TRANSPARENT, WIDTH},
};
fn real() -> Snapshot {
    Snapshot::from_json(include_bytes!("fixtures/real-town.json")).unwrap()
}
fn measures(plan: &CityPlan, slot: usize) -> (f64, usize, f64) {
    let (dx, dy) = DIRECTIONS[slot];
    let norm = ((dx * dx + dy * dy) as f64).sqrt();
    let arm = &plan.arms[slot];
    let length = arm
        .land
        .iter()
        .map(|&(x, y)| ((x - 56) * dx + (y - 56) * dy) as f64 / norm)
        .fold(0.0, f64::max);
    let mut low = 0.0_f64;
    let mut high = 0.0_f64;
    for &(x, y) in &arm.land {
        let n = (-(x - 56) * dy + (y - 56) * dx) as f64 / norm;
        low = low.min(n);
        high = high.max(n);
    }
    (length, arm.land.len(), high - low)
}
#[test]
fn real_town_density_and_shape() {
    let s = real();
    let plan = CityPlan::new(&s);
    println!("buildings {}", plan.buildings.len());
    for a in &plan.arms {
        println!(
            "{} slot {} count {} metrics {:?}",
            a.language,
            a.slot,
            a.plots.len(),
            measures(&plan, a.slot)
        );
    }
    for (i, l) in s.languages.iter().enumerate() {
        println!(
            "{} buildings {}",
            l.name,
            plan.buildings.iter().filter(|b| b.district == i).count()
        );
    }
    assert!((40..=60).contains(&plan.buildings.len()));
    let mut counts: Vec<_> = plan
        .arms
        .iter()
        .map(|a| {
            (
                s.languages
                    .iter()
                    .find(|l| l.name == a.language)
                    .unwrap()
                    .commits,
                a.plots.len(),
            )
        })
        .collect();
    counts.sort();
    assert!(counts.windows(2).all(|p| p[0].1 < p[1].1));
    let py = plan
        .arms
        .iter()
        .position(|a| a.language == "Python")
        .unwrap();
    let ts = plan
        .arms
        .iter()
        .position(|a| a.language == "TypeScript")
        .unwrap();
    let a = measures(&plan, py);
    let b = measures(&plan, ts);
    assert!(b.0 >= a.0 * 1.3);
    assert!(b.1 as f64 >= a.1 as f64 * 1.6);
    for arm in &plan.arms {
        let m = measures(&plan, arm.slot);
        assert!(m.0 >= 2.5 * m.2, "{} {:?}", arm.language, m);
    }
}
#[test]
fn slab_columns_are_solid_except_true_coves() {
    let plan = CityPlan::new(&real());
    let f = bare_ground(&plan, Setting::Earth);
    // Independently project the four corners of every solid tile column. Gaps outside this union are genuine coves, rather than cracks in a rendered face.
    let mut expected = vec![false; WIDTH * HEIGHT];
    for &(x, y) in &plan.land {
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (sx, sy) = git_village::voxel::project(x + dx, y + dy, 0);
            for py in sy..=sy + 10 {
                if sx >= 0 && sx < WIDTH as i32 && py >= 0 && py < HEIGHT as i32 {
                    expected[py as usize * WIDTH + sx as usize] = true;
                }
            }
        }
    }
    for (i, needed) in expected.iter().enumerate() {
        if *needed {
            assert_ne!(
                f.pixels[i],
                TRANSPARENT,
                "slab hole at {},{}",
                i % WIDTH,
                i / WIDTH
            );
        }
    }
}
#[test]
fn weekly_cranes_follow_work() {
    for stage in 0..6 {
        for (work, expected) in [
            (0, 0),
            (1, 1),
            (4, 1),
            (9, 1),
            (10, 2),
            (49, 2),
            (50, 3),
            (499, 3),
        ] {
            assert_eq!(crane_count(work, stage), expected);
        }
        assert_eq!(crane_count(500, stage), if stage >= 4 { 4 } else { 3 });
    }
}

#[test]
fn a_mid_arm_circle_counts_four_separate_peninsulas() {
    let plan = CityPlan::new(&real());
    let radius = 66.0;
    let ring: Vec<_> = (0..4096)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::TAU / 4096.0;
            plan.land.contains(&(
                (56.0 + radius * angle.cos()).round() as i32,
                (56.0 + radius * angle.sin()).round() as i32,
            ))
        })
        .collect();
    let transitions = (0..ring.len())
        .filter(|&i| !ring[i] && ring[(i + 1) % ring.len()])
        .count();
    assert_eq!(transitions, 4);
}
#[test]
fn arms_remain_long_at_one_hundred_commits_and_at_maximum() {
    for commits in [100, 253, 675, 1000, 100000] {
        let mut s = real();
        s.total_commits = commits * 8;
        s.languages = (0..8)
            .map(|i| git_village::snapshot::Language {
                name: format!("L{i}"),
                color: None,
                commits,
                arm_earned_on: Some(s.created_on + chrono::Duration::days(i)),
            })
            .collect();
        let plan = CityPlan::new(&s);
        for slot in 0..8 {
            let m = measures(&plan, slot);
            assert!(m.0 >= 2.5 * m.2, "{commits} slot {slot}: {m:?}");
        }
    }
}

#[test]
fn small_hub_languages_are_small_and_arm_languages_keep_their_frontages() {
    use git_village::{
        layout::Archetype,
        render::{Backdrop, Renderer},
    };
    let s = real();
    let plan = CityPlan::new(&s);
    let renderer = Renderer::with_backdrop(&s, Setting::Earth, Backdrop::None).unwrap();
    assert_eq!(renderer.crane_count(), 1);
    for (index, language) in s
        .languages
        .iter()
        .enumerate()
        .filter(|(_, l)| l.commits < 25)
    {
        let count = plan
            .buildings
            .iter()
            .filter(|b| b.district == index)
            .count();
        assert_eq!(count, git_village::plan::district_count(language.commits));
        assert!(count <= 3);
    }
    assert!(
        plan.buildings
            .iter()
            .any(|b| b.district == usize::MAX && b.color == [154, 158, 146])
    );
    for b in &plan.buildings {
        let extra = match b.archetype {
            Archetype::Tower => {
                if b.width < 6 {
                    3
                } else {
                    5
                }
            }
            Archetype::Foundry => 3,
            Archetype::Academy => {
                if b.width < 6 {
                    2
                } else {
                    4
                }
            }
            Archetype::Temple => {
                if b.width < 6 {
                    3
                } else {
                    7
                }
            }
            _ => 0,
        };
        assert!(b.height + extra <= b.width * 2, "Town pencil: {b:?}");
    }
    for arm in &plan.arms {
        for &(x, y) in &arm.plots {
            let b = plan
                .buildings
                .iter()
                .find(|b| b.x == x && b.y == y)
                .unwrap();
            assert_eq!(s.languages[b.district].name, arm.language);
        }
    }
}
#[test]
fn cropped_exports_share_one_loop_box_with_exactly_four_pixels_of_margin() {
    use git_village::{
        motion::FRAME_COUNT,
        output::{encode_gif, encode_png_with_bounds, export_bounds},
        render::{Backdrop, Renderer},
    };
    let renderer = Renderer::with_backdrop(&real(), Setting::Earth, Backdrop::None).unwrap();
    let frames: Vec<_> = (0..FRAME_COUNT).map(|f| renderer.frame(f)).collect();
    let bounds = export_bounds(&frames);
    assert_eq!(bounds, export_bounds(&frames));
    let mut extrema = (usize::MAX, usize::MAX, 0, 0);
    for f in &frames {
        for (i, p) in f.pixels.iter().enumerate() {
            if *p != TRANSPARENT {
                let (x, y) = (i % WIDTH, i / WIDTH);
                extrema.0 = extrema.0.min(x);
                extrema.1 = extrema.1.min(y);
                extrema.2 = extrema.2.max(x);
                extrema.3 = extrema.3.max(y);
            }
        }
    }
    assert_eq!(extrema.0 - bounds.left, 4);
    assert_eq!(extrema.1 - bounds.top, 4);
    assert_eq!(bounds.left + bounds.width - 1 - extrema.2, 4);
    assert_eq!(bounds.top + bounds.height - 1 - extrema.3, 4);
    for f in &frames {
        assert_eq!(f.pixels[bounds.top * WIDTH + bounds.left], TRANSPARENT);
    }
    let gif = encode_gif(&frames).unwrap();
    let decoder = gif::DecodeOptions::new().read_info(&gif[..]).unwrap();
    let png = encode_png_with_bounds(&frames[0], bounds).unwrap();
    let png = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    assert_eq!(
        (decoder.width() as u32, decoder.height() as u32),
        (png.info().width, png.info().height)
    );
    let scene = Renderer::with_backdrop(&real(), Setting::Earth, Backdrop::Scene)
        .unwrap()
        .still();
    let scene_bounds = export_bounds(&[scene]);
    assert_eq!((scene_bounds.width, scene_bounds.height), (WIDTH, HEIGHT));
    assert!(!git_village::output::encode_preview_html().contains("width=\""));
}
