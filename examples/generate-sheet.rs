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
                        // A city wider than the base canvas shows its center here.
                        let shifted_x = x + frame.width.saturating_sub(WIDTH) / 2;
                        let shifted_y = y + frame.height.saturating_sub(HEIGHT) / 2;
                        let color = frame.pixels[shifted_y.min(frame.height - 1) * frame.width
                            + shifted_x.min(frame.width - 1)];
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
            "{stage}: buildings {} blocks {}",
            plan.buildings.len(),
            plan.walks.len() - 1
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
        let (label_x, label_y) = (still.left + 4, still.top + 4);
        text(&mut still, label_x, label_y, stage, [255, 245, 204]);
        stages.push(still);
    }
    std::fs::write(format!("{destination}/growth-sheet.png"), sheet(&stages)?)?;
    // One still per setting, from an account at the City stage, in the backdrop that shows it best.
    let mut account = synthetic(20000);
    account.daily_commits.clear();
    account.daily_commits.insert(account.observed_on, 3);
    account
        .daily_commits
        .insert(account.observed_on - Duration::days(1), 3);
    for setting in Setting::ALL {
        let renderer = Renderer::new(&account, setting)?;
        let frames: Vec<_> = (0..FRAME_COUNT)
            .step_by(96)
            .map(|i| renderer.frame(i))
            .collect();
        std::fs::write(
            format!("{destination}/setting-{}.png", setting.name()),
            encode_png_with_bounds(&frames[0], export_bounds(&frames))?,
        )?;
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
        std::fs::write(format!("{destination}/{mood:?}.png"), encode_png(&still)?)?;
        let (label_x, label_y) = (still.left + 4, still.top + 4);
        text(
            &mut still,
            label_x,
            label_y,
            &format!("{mood:?}"),
            [255, 245, 204],
        );
        moods.push(still);
    }
    std::fs::write(format!("{destination}/mood-sheet.png"), sheet(&moods)?)?;
    Ok(())
}
