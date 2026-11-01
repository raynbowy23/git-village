#[path = "../tests/fixtures/support.rs"]
mod support;
use support::synthetic;
// Doubled review sheets and one complete loop per growth stage.
use chrono::Duration;
use git_village::{
    development::STAGES,
    mood::{Mood, select},
    motion::FRAME_COUNT,
    output::{encode_gif, encode_png, encode_png_with_bounds, export_bounds},
    render::{Backdrop, Renderer, text},
    setting::Setting,
    snapshot::Snapshot,
    voxel::{Frame, HEIGHT, WIDTH},
};
fn sheet(frames: &[Frame]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let columns = 3;
    let rows = frames.len().div_ceil(columns);
    let width = WIDTH * columns * 2;
    let height = HEIGHT * rows * 2;
    let mut pixels = vec![0; width * height * 3];
    for (index, frame) in frames.iter().enumerate() {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                for dy in 0..2 {
                    for dx in 0..2 {
                        let offset = (((index / columns * HEIGHT + y) * 2 + dy) * width
                            + (index % columns * WIDTH + x) * 2
                            + dx)
                            * 3;
                        let color = frame.pixels[y * WIDTH + x];
                        pixels[offset..offset + 3].copy_from_slice(&if color
                            == git_village::voxel::TRANSPARENT
                        {
                            if (x / 8 + y / 8) % 2 == 0 {
                                [235, 235, 235]
                            } else {
                                [218, 218, 218]
                            }
                        } else {
                            color
                        });
                    }
                }
            }
        }
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&pixels)?;
    }
    Ok(bytes)
}
fn mood_snapshot(mood: Mood) -> Snapshot {
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
    // Keep language weights valid when selecting the lower milestone.
    for language in &mut snapshot.languages {
        language.commits = snapshot.total_commits / 5;
    }
    assert_eq!(select(&snapshot), mood);
    snapshot
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example generate-sheet -- DIRECTORY")?;
    std::fs::create_dir_all(&destination)?;
    if let Some(path) = std::env::args().nth(2) {
        let snapshot = Snapshot::from_json(&std::fs::read(path)?)?;
        for backdrop in [Backdrop::None, Backdrop::Scene] {
            let renderer = Renderer::with_backdrop(&snapshot, Setting::Earth, backdrop)?;
            let frames: Vec<_> = (0..FRAME_COUNT).map(|f| renderer.frame(f)).collect();
            let bounds = export_bounds(&frames);
            let gif = encode_gif(&frames)?;
            println!(
                "Real {backdrop:?}: {} bytes, {}x{} pixels",
                gif.len(),
                bounds.width * 2,
                bounds.height * 2
            );
            std::fs::write(format!("{destination}/real-{backdrop:?}.gif"), gif)?;
            std::fs::write(
                format!("{destination}/real-{backdrop:?}.png"),
                encode_png_with_bounds(&frames[0], bounds)?,
            )?;
            if backdrop == Backdrop::None {
                for (label, background) in [("white", [255, 255, 255]), ("dark", [13, 17, 23])] {
                    let mut matte = frames[0].clone();
                    for p in &mut matte.pixels {
                        if *p == git_village::voxel::TRANSPARENT {
                            *p = background;
                        }
                    }
                    std::fs::write(
                        format!("{destination}/real-{label}.png"),
                        encode_png_with_bounds(&matte, bounds)?,
                    )?;
                    let mut silhouette = git_village::render::bare_ground(
                        &git_village::plan::CityPlan::new(&snapshot),
                        Setting::Earth,
                    );
                    for p in &mut silhouette.pixels {
                        *p = if *p == git_village::voxel::TRANSPARENT {
                            background
                        } else if label == "white" {
                            [35, 44, 38]
                        } else {
                            [210, 218, 209]
                        };
                    }
                    std::fs::write(
                        format!("{destination}/silhouette-{label}.png"),
                        encode_png_with_bounds(&silhouette, bounds)?,
                    )?;
                }
            }
        }
    }
    let mut stages = Vec::new();
    for (stage, total) in STAGES {
        let mut snapshot = synthetic(total);
        snapshot.daily_commits.clear();
        if total > 0 {
            snapshot
                .daily_commits
                .insert(snapshot.observed_on, total.min(300));
        }
        let plan = git_village::plan::CityPlan::new(&snapshot);
        println!(
            "{stage}: buildings {} arms {:?} walks {:?}",
            plan.buildings.len(),
            plan.arms
                .iter()
                .map(|a| (&a.language, a.land.len(), a.plots.len()))
                .collect::<Vec<_>>(),
            plan.walks.iter().map(|r| r.district).collect::<Vec<_>>()
        );
        let renderer = Renderer::new(&snapshot, Setting::Earth)?;
        let frames: Vec<_> = (0..FRAME_COUNT)
            .map(|index| renderer.frame(index))
            .collect();
        let gif = encode_gif(&frames)?;
        println!("{stage}: {} bytes, {} frames", gif.len(), frames.len());
        std::fs::write(format!("{destination}/{stage}.gif"), gif)?;
        let mut still = renderer.still();
        std::fs::write(
            format!("{destination}/{stage}.png"),
            encode_png_with_bounds(&still, export_bounds(&frames))?,
        )?;
        text(&mut still, 4, 4, stage, [255, 245, 204]);
        stages.push(still);
    }
    std::fs::write(format!("{destination}/growth-sheet.png"), sheet(&stages)?)?;
    let mut realistic = Vec::new();
    for (stage, total) in STAGES {
        let mut snapshot = synthetic(total);
        for (language, weight) in snapshot.languages.iter_mut().zip([45, 25, 15, 10, 5]) {
            language.commits = total / 100 * weight;
            language.arm_earned_on = (language.commits >= 25)
                .then(|| snapshot.created_on + Duration::days(2500 / weight as i64));
        }
        let renderer = Renderer::with_backdrop(&snapshot, Setting::Earth, Backdrop::None)?;
        let mut frame = renderer.still();
        text(&mut frame, 4, 4, stage, [70, 70, 70]);
        realistic.push(frame);
    }
    std::fs::write(
        format!("{destination}/language-growth-sheet.png"),
        sheet(&realistic)?,
    )?;
    let mut arms = Vec::new();
    for count in [1, 3, 5, 8] {
        let mut snapshot = synthetic(100000);
        let names = [
            "Rust",
            "Python",
            "TypeScript",
            "Go",
            "Ruby",
            "Shell",
            "Haskell",
            "Java",
        ];
        snapshot.languages = (0..count)
            .map(|i| git_village::snapshot::Language {
                name: names[i].into(),
                color: None,
                commits: 100000 / count as u64,
                arm_earned_on: Some(snapshot.created_on + Duration::days(i as i64)),
            })
            .collect();
        let renderer = Renderer::with_backdrop(&snapshot, Setting::Earth, Backdrop::None)?;
        let mut frame = renderer.still();
        text(&mut frame, 4, 4, &format!("{count} ARMS"), [255, 245, 204]);
        arms.push(frame);
    }
    std::fs::write(format!("{destination}/arms-sheet.png"), sheet(&arms)?)?;
    let mut maximum = synthetic(800000);
    maximum.languages = (0..8)
        .map(|i| git_village::snapshot::Language {
            name: format!("Language{i}"),
            color: None,
            commits: 100000,
            arm_earned_on: Some(maximum.created_on + Duration::days(i)),
        })
        .collect();
    maximum.daily_commits.clear();
    maximum.daily_commits.insert(maximum.observed_on, 799999);
    maximum
        .daily_commits
        .insert(maximum.observed_on - Duration::days(1), 1);
    for backdrop in [Backdrop::None, Backdrop::Scene] {
        let renderer = Renderer::with_backdrop(&maximum, Setting::Earth, backdrop)?;
        let frames: Vec<_> = (0..FRAME_COUNT).map(|f| renderer.frame(f)).collect();
        let gif = encode_gif(&frames)?;
        println!(
            "Maximum eight-arm Festival {backdrop:?}: {} bytes, {}x{} pixels",
            gif.len(),
            export_bounds(&frames).width * 2,
            export_bounds(&frames).height * 2
        );
        assert!(gif.len() <= 4 * 1024 * 1024);
        std::fs::write(format!("{destination}/maximum-{backdrop:?}.gif"), gif)?;
    }
    let mut moods = Vec::new();
    for mood in [
        Mood::Abandoned,
        Mood::Night,
        Mood::MarketDay,
        Mood::Festival,
        Mood::Workday,
    ] {
        let snapshot = mood_snapshot(mood);
        let renderer = Renderer::new(&snapshot, Setting::Earth)?;
        let mut still = renderer.still();
        text(&mut still, 4, 4, &format!("{mood:?}"), [255, 245, 204]);
        std::fs::write(format!("{destination}/{mood:?}.png"), encode_png(&still)?)?;
        moods.push(still);
    }
    std::fs::write(format!("{destination}/mood-sheet.png"), sheet(&moods)?)?;
    Ok(())
}
