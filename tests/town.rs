use git_village::{
    plan::CityPlan,
    render::{bare_ground, crane_count},
    setting::Setting,
    snapshot::Snapshot,
    voxel::{HEIGHT, TRANSPARENT, WIDTH},
};
fn real() -> Snapshot {
    Snapshot::from_json(include_bytes!("fixtures/real-town.json")).unwrap()
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
