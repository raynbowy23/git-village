//! The bare island: real transparency that survives a round trip through the encoders.
use git_village::{
    motion::FRAME_COUNT,
    output::{encode_gif, encode_png_with_bounds, export_bounds},
    render::{Backdrop, Renderer},
    setting::Setting,
    snapshot::Snapshot,
    voxel::{Frame, TRANSPARENT},
};
use std::io::Cursor;

const FESTIVAL: &[u8] = include_bytes!("fixtures/festival.json");
const OUTPOST: &[u8] = include_bytes!("fixtures/outpost.json");

fn loop_frames(fixture: &[u8], backdrop: Backdrop) -> Vec<Frame> {
    let snapshot = Snapshot::from_json(fixture).unwrap();
    let renderer = Renderer::with_backdrop(&snapshot, Setting::Earth, backdrop).unwrap();
    (0..FRAME_COUNT)
        .map(|index| renderer.frame(index))
        .collect()
}

#[test]
fn the_scene_never_draws_the_transparent_key_color() {
    for frame in loop_frames(FESTIVAL, Backdrop::Scene).iter().step_by(17) {
        assert!(!frame.pixels.contains(&TRANSPARENT));
    }
}

#[test]
fn decoded_cutout_gif_matches_every_source_frame() {
    assert_round_trip(FESTIVAL);
    assert_round_trip(OUTPOST);
}

/// Transparency must be exact in every frame. Colors must also be exact when the loop fits the palette; the busiest fixture exceeds 255 colors, and its rarest colors map to their nearest entry.
fn assert_round_trip(fixture: &[u8]) {
    let frames = loop_frames(fixture, Backdrop::None);
    let unique: std::collections::BTreeSet<_> = frames
        .iter()
        .flat_map(|frame| frame.pixels.iter().copied())
        .filter(|&color| color != TRANSPARENT)
        .collect();
    let exact_colors = unique.len() <= 255;
    let bytes = encode_gif(&frames).unwrap();

    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::Indexed);
    let mut decoder = options.read_info(Cursor::new(&bytes)).unwrap();
    let palette = decoder.global_palette().unwrap().to_vec();
    let bounds = export_bounds(&frames);
    let (width, height) = (bounds.width * 2, bounds.height * 2);
    assert_eq!(
        (decoder.width() as usize, decoder.height() as usize),
        (width, height)
    );
    let mut canvas = vec![0_u8; width * height];
    let mut shown = 0;
    let mut duration = 0_u32;
    while let Some(decoded) = decoder.read_next_frame().unwrap() {
        duration += u32::from(decoded.delay);
        let saved = canvas.clone();
        let (left, top) = (decoded.left as usize, decoded.top as usize);
        for (offset, &value) in decoded.buffer.iter().enumerate() {
            if Some(value) != decoded.transparent {
                let x = left + offset % decoded.width as usize;
                let y = top + offset / decoded.width as usize;
                canvas[y * width + x] = value;
            }
        }
        // The first decoded frame is the base; the loop's frames follow it.
        if shown > 0 {
            let source = &frames[shown - 1];
            for y in 0..source.height {
                for x in 0..source.width {
                    let color = source.pixels[y * source.width + x];
                    if x < bounds.left
                        || y < bounds.top
                        || x >= bounds.left + bounds.width
                        || y >= bounds.top + bounds.height
                    {
                        assert_eq!(color, TRANSPARENT);
                        continue;
                    }
                    let value =
                        canvas[(y - bounds.top) * 2 * width + (x - bounds.left) * 2] as usize;
                    if color == TRANSPARENT {
                        assert_eq!(value, 0, "frame {shown} leaves a mark at {x},{y}");
                    } else {
                        assert_ne!(value, 0, "frame {shown} has a hole at {x},{y}");
                        if exact_colors {
                            assert_eq!(palette[value * 3..value * 3 + 3], color, "frame {shown}");
                        }
                    }
                }
            }
        }
        if decoded.dispose == gif::DisposalMethod::Previous {
            canvas = saved;
        }
        shown += 1;
    }
    assert_eq!(shown, FRAME_COUNT + 1);
    assert_eq!(duration as usize, FRAME_COUNT * 100 / 12);
}

#[test]
fn cutout_png_has_alpha_with_a_clear_corner_and_an_opaque_island() {
    let frames = loop_frames(FESTIVAL, Backdrop::None);
    let frame = &frames[0];
    let bounds = export_bounds(&frames);
    let bytes = encode_png_with_bounds(frame, bounds).unwrap();
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(pixels[3], 0);
    let (x, y) = git_village::voxel::project(56, 56, 0);
    let center =
        ((y as usize - bounds.top) * 2 * bounds.width * 2 + (x as usize - bounds.left) * 2) * 4;
    assert_eq!(
        (info.width as usize, info.height as usize),
        (bounds.width * 2, bounds.height * 2)
    );
    assert_eq!(pixels[center + 3], 255);
}
