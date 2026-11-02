//! Deterministic encoders and temporary-sibling file replacement.
use crate::voxel::{Frame, TRANSPARENT};
use std::{borrow::Cow, collections::BTreeMap, path::Path};
/// One native-pixel export rectangle, shared by every loop frame and its still.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportBounds {
    /// Left native column.
    pub left: usize,
    /// Top native row.
    pub top: usize,
    /// Native width before the unchanged 2x export.
    pub width: usize,
    /// Native height before the unchanged 2x export.
    pub height: usize,
    /// Columns in the source frame's buffer.
    pub stride: usize,
}
impl ExportBounds {
    fn source(self, index: usize) -> usize {
        (self.top + index / self.width) * self.stride + self.left + index % self.width
    }
}
/// Union all opaque loop pixels and add four native pixels on each side; an opaque scene retains the full canvas.
pub fn export_bounds(frames: &[Frame]) -> ExportBounds {
    let (stride, rows) = (frames[0].width, frames[0].height);
    let full = ExportBounds {
        left: 0,
        top: 0,
        width: stride,
        height: rows,
        stride,
    };
    if !frames.iter().any(|f| f.pixels.contains(&TRANSPARENT)) {
        return full;
    }
    let mut bounds = None;
    for frame in frames {
        for (i, color) in frame.pixels.iter().enumerate() {
            if *color != TRANSPARENT {
                let (x, y) = (i % stride, i / stride);
                let (left, top, right, bottom) = bounds.get_or_insert((x, y, x, y));
                *left = (*left).min(x);
                *top = (*top).min(y);
                *right = (*right).max(x);
                *bottom = (*bottom).max(y);
            }
        }
    }
    let Some((left, top, right, bottom)) = bounds else {
        return ExportBounds {
            left: 0,
            top: 0,
            width: 8,
            height: 8,
            stride,
        };
    };
    let left = left.saturating_sub(4);
    let top = top.saturating_sub(4);
    ExportBounds {
        left,
        top,
        width: (right + 5).min(stride) - left,
        height: (bottom + 5).min(rows) - top,
        stride,
    }
}
fn cropped_pixels(frame: &Frame, bounds: ExportBounds) -> impl Iterator<Item = [u8; 3]> + '_ {
    (0..bounds.width * bounds.height).map(move |i| frame.pixels[bounds.source(i)])
}
/// Encode doubled nearest-neighbor PNG, with an alpha channel when the frame has transparent pixels.
pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, png::EncodingError> {
    encode_png_with_bounds(frame, export_bounds(std::slice::from_ref(frame)))
}
/// Encode the still using the loop's export rectangle, preserving identical PNG and GIF dimensions.
pub fn encode_png_with_bounds(
    frame: &Frame,
    bounds: ExportBounds,
) -> Result<Vec<u8>, png::EncodingError> {
    let cutout = frame.pixels.contains(&TRANSPARENT);
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(
            &mut bytes,
            (bounds.width * 2) as u32,
            (bounds.height * 2) as u32,
        );
        encoder.set_color(if cutout {
            png::ColorType::Rgba
        } else {
            png::ColorType::Rgb
        });
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        let mut pixels = Vec::with_capacity(bounds.width * bounds.height * 16);
        for y in bounds.top..bounds.top + bounds.height {
            let row = &frame.pixels
                [y * frame.width + bounds.left..y * frame.width + bounds.left + bounds.width];
            for _ in 0..2 {
                for color in row {
                    for _ in 0..2 {
                        pixels.extend_from_slice(color);
                        if cutout {
                            pixels.push(if *color == TRANSPARENT { 0 } else { 255 });
                        }
                    }
                }
            }
        }
        writer.write_image_data(&pixels)?;
    }
    Ok(bytes)
}
/// The most a GIF may weigh, in bytes. A profile image is fetched on every page view and committed to a repository's history.
pub const GIF_BUDGET: usize = 1_000_000;
/// Shares of walkers, vehicles and animated buildings to try, stopping at the first loop inside the budget.
const DETAIL_LEVELS: [f64; 9] = [1.0, 0.7, 0.5, 0.35, 0.25, 0.15, 0.08, 0.03, 0.0];
/// Render a loop and encode it inside [`GIF_BUDGET`], thinning the motion only as far as needed. The same snapshot always lands on the same level.
pub fn encode_within_budget(
    renderer: crate::render::Renderer,
) -> Result<(Vec<Frame>, Vec<u8>), gif::EncodingError> {
    let mut renderer = renderer;
    let mut last = None;
    for detail in DETAIL_LEVELS {
        renderer = renderer.with_detail(detail);
        let frames: Vec<_> = (0..crate::motion::FRAME_COUNT)
            .map(|index| renderer.frame(index))
            .collect();
        let gif = encode_gif(&frames)?;
        let fits = gif.len() <= GIF_BUDGET;
        last = Some((frames, gif));
        if fits {
            break;
        }
    }
    Ok(last.expect("the detail levels are not empty"))
}
/// How long the bare base frame of a cutout GIF shows, in hundredths of a second. Browsers honor two and slow anything shorter.
const BASE_FRAME_DELAY: u16 = 2;
/// Hundredths of a second that one loop frame shows, as the step in a rounded running total, so a loop lasts exactly its frame count divided by the frame rate.
fn frame_delay(frame_index: usize) -> u16 {
    (((frame_index + 1) * 100 + 6) / 12 - frame_index.saturating_mul(100).saturating_add(6) / 12)
        as u16
}
/// Encode a loop with a global palette. A scene stores changed-pixel boxes with Keep disposal, where transparent means unchanged. A frame with transparent pixels cannot use that, because Keep can never erase a pixel back to the page, so a cutout goes through [`encode_cutout_gif`].
pub fn encode_gif(frames: &[Frame]) -> Result<Vec<u8>, gif::EncodingError> {
    let bounds = export_bounds(frames);
    let width = bounds.width;
    let cutout = frames
        .iter()
        .any(|frame| frame.pixels.contains(&TRANSPARENT));
    let mut frequencies = BTreeMap::<[u8; 3], usize>::new();
    for frame in frames {
        for color in frame.pixels.iter().filter(|&&color| color != TRANSPARENT) {
            *frequencies.entry(*color).or_default() += 1;
        }
    }
    let mut colors: Vec<_> = frequencies.into_iter().collect();
    colors.sort_by_key(|&(color, count)| (std::cmp::Reverse(count), color));
    colors.truncate(255);
    let selected: Vec<_> = colors.into_iter().map(|(color, _)| color).collect();
    let mut palette = vec![0, 0, 0];
    for color in &selected {
        palette.extend_from_slice(color);
    }
    let mut indices = BTreeMap::new();
    for frame in frames {
        for &color in frame.pixels.iter().filter(|&&color| color != TRANSPARENT) {
            indices.entry(color).or_insert_with(|| {
                (1 + selected
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, candidate)| {
                        color
                            .iter()
                            .zip(candidate.iter())
                            .map(|(&a, &b)| (i32::from(a) - i32::from(b)).pow(2))
                            .sum::<i32>()
                    })
                    .map_or(0, |(index, _)| index)) as u8
            });
        }
    }
    let mut encoder = gif::Encoder::new(
        Vec::new(),
        (bounds.width * 2) as u16,
        (bounds.height * 2) as u16,
        &palette,
    )?;
    encoder.set_repeat(gif::Repeat::Infinite)?;
    if cutout {
        return encode_cutout_gif(encoder, frames, &indices, bounds);
    }
    let mut previous = vec![0; bounds.width * bounds.height];
    for (frame_index, frame) in frames.iter().enumerate() {
        let current: Vec<_> = cropped_pixels(frame, bounds)
            .map(|color| indices[&color])
            .collect();
        let mut bounds = None;
        for (i, (&now, &before)) in current.iter().zip(&previous).enumerate() {
            if frame_index == 0 || now != before {
                let x = i % width;
                let y = i / width;
                let (left, top, right, bottom) = bounds.get_or_insert((x, y, x, y));
                *left = (*left).min(x);
                *top = (*top).min(y);
                *right = (*right).max(x);
                *bottom = (*bottom).max(y);
            }
        }
        let (left, top, right, bottom) = bounds.unwrap_or((0, 0, 0, 0));
        let mut pixels = Vec::new();
        for y in top..=bottom {
            let mut row = Vec::new();
            for x in left..=right {
                let i = y * width + x;
                let value = if frame_index > 0 && current[i] == previous[i] {
                    0
                } else {
                    current[i]
                };
                row.extend([value, value]);
            }
            pixels.extend_from_slice(&row);
            pixels.extend_from_slice(&row);
        }
        encoder.write_frame(&gif::Frame {
            left: (left * 2) as u16,
            top: (top * 2) as u16,
            width: ((right - left + 1) * 2) as u16,
            height: ((bottom - top + 1) * 2) as u16,
            delay: frame_delay(frame_index),
            dispose: gif::DisposalMethod::Keep,
            transparent: Some(0),
            buffer: Cow::Owned(pixels),
            ..Default::default()
        })?;
        previous = current;
    }
    encoder.into_inner()
}
/// Encode a cutout: first a base frame of every pixel that never changes, kept for the whole loop, then each loop frame as just its differences from the base, restored with Previous disposal. Moving pixels can therefore pass over the empty page and leave nothing behind, without storing the whole city in every frame.
fn encode_cutout_gif(
    mut encoder: gif::Encoder<Vec<u8>>,
    frames: &[Frame],
    indices: &BTreeMap<[u8; 3], u8>,
    bounds: ExportBounds,
) -> Result<Vec<u8>, gif::EncodingError> {
    let index_of = |color: [u8; 3]| {
        if color == TRANSPARENT {
            0
        } else {
            indices[&color]
        }
    };
    let base: Vec<u8> = (0..bounds.width * bounds.height)
        .map(|i| {
            let color = frames[0].pixels[bounds.source(i)];
            if frames
                .iter()
                .all(|frame| frame.pixels[bounds.source(i)] == color)
            {
                index_of(color)
            } else {
                0
            }
        })
        .collect();
    write_doubled_box(
        &mut encoder,
        &base,
        bounds.width,
        BASE_FRAME_DELAY,
        gif::DisposalMethod::Keep,
    )?;
    for (frame_index, frame) in frames.iter().enumerate() {
        let difference: Vec<u8> = cropped_pixels(frame, bounds)
            .zip(&base)
            .map(|(color, &under)| {
                let index = index_of(color);
                if index == under { 0 } else { index }
            })
            .collect();
        let delay = frame_delay(frame_index)
            - if frame_index == 0 {
                BASE_FRAME_DELAY
            } else {
                0
            };
        write_doubled_box(
            &mut encoder,
            &difference,
            bounds.width,
            delay,
            gif::DisposalMethod::Previous,
        )?;
    }
    encoder.into_inner()
}
/// Write the box around a native index buffer's drawn pixels at export scale. An empty buffer still writes one transparent pixel, since a GIF frame cannot be empty.
fn write_doubled_box(
    encoder: &mut gif::Encoder<Vec<u8>>,
    native: &[u8],
    width: usize,
    delay: u16,
    dispose: gif::DisposalMethod,
) -> Result<(), gif::EncodingError> {
    let mut bounds = None;
    for (i, _) in native.iter().enumerate().filter(|&(_, &value)| value != 0) {
        let (x, y) = (i % width, i / width);
        let (left, top, right, bottom) = bounds.get_or_insert((x, y, x, y));
        *left = (*left).min(x);
        *top = (*top).min(y);
        *right = (*right).max(x);
        *bottom = (*bottom).max(y);
    }
    let (left, top, right, bottom) = bounds.unwrap_or((0, 0, 0, 0));
    let mut pixels = Vec::new();
    for y in top..=bottom {
        let mut row = Vec::new();
        for x in left..=right {
            row.extend([native[y * width + x]; 2]);
        }
        pixels.extend_from_slice(&row);
        pixels.extend_from_slice(&row);
    }
    encoder.write_frame(&gif::Frame {
        left: (left * 2) as u16,
        top: (top * 2) as u16,
        width: ((right - left + 1) * 2) as u16,
        height: ((bottom - top + 1) * 2) as u16,
        delay,
        dispose,
        transparent: Some(0),
        buffer: Cow::Owned(pixels),
        ..Default::default()
    })
}
/// Preview the sibling exports without scripts or external dependencies.
pub fn encode_preview_html() -> String {
    format!(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>{name}</title><style>body{{margin:0;font:14px monospace}}section{{padding:16px}}.light{{background:#fff;color:#1f2328}}.dark{{background:#0d1117;color:#f0f6fc}}img{{image-rendering:pixelated;max-width:100%}}</style><section class=\"light\"><h1>{name}</h1><p>GitHub light</p><img src=\"{gif}\" alt=\"Animated commit village\"><img src=\"{png}\" alt=\"Commit village still\"></section><section class=\"dark\"><p>GitHub dark</p><img src=\"{gif}\" alt=\"Animated commit village\"><img src=\"{png}\" alt=\"Commit village still\"></section></html>",
        name = crate::DISPLAY_NAME,
        gif = crate::GIF_FILE,
        png = crate::PNG_FILE
    )
}
/// Write all temporary siblings before replacing destination files.
pub fn replace_files(directory: &Path, files: &BTreeMap<&str, Vec<u8>>) -> std::io::Result<()> {
    std::fs::create_dir_all(directory)?;
    let mut temporary = Vec::new();
    for (name, bytes) in files {
        let path = directory.join(format!(".{name}.{}.tmp", std::process::id()));
        if let Err(error) = std::fs::write(&path, bytes) {
            for path in temporary {
                let _ = std::fs::remove_file(path);
            }
            return Err(error);
        }
        temporary.push(path);
    }
    for ((name, _), temporary_path) in files.iter().zip(temporary) {
        std::fs::rename(temporary_path, directory.join(name))?;
    }
    Ok(())
}
