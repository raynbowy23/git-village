//! Integer isometric rasterization with per-pixel depth.
/// Native width.
pub const WIDTH: usize = 384;
/// Native height.
pub const HEIGHT: usize = 256;
/// Key color for pixels with no scene behind them. The encoders turn it into real transparency, so no scene may draw this exact color.
pub const TRANSPARENT: [u8; 3] = [255, 0, 254];
/// RGB frame and depth buffer; larger depth is nearer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Opaque RGB pixels.
    pub pixels: Vec<[u8; 3]>,
    /// Occlusion depths.
    pub depth: Vec<i32>,
}
/// Isometric 2:1 world projection.
pub fn project(x: i32, y: i32, z: i32) -> (i32, i32) {
    (192 + x - y, 78 + (x + y) / 2 - z)
}
impl Frame {
    /// Fill a frame with one color.
    pub fn new(color: [u8; 3]) -> Self {
        Self {
            pixels: vec![color; WIDTH * HEIGHT],
            depth: vec![i32::MIN; WIDTH * HEIGHT],
        }
    }
    /// Depth-tested pixel.
    pub fn pixel(&mut self, x: i32, y: i32, depth: i32, color: [u8; 3]) {
        if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
            let index = y as usize * WIDTH + x as usize;
            if depth >= self.depth[index] {
                self.pixels[index] = color;
                self.depth[index] = depth
            }
        }
    }
    /// Screen rectangle at fixed depth.
    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, depth: i32, color: [u8; 3]) {
        for row in y..y + height {
            for column in x..x + width {
                self.pixel(column, row, depth, color)
            }
        }
    }
    /// Integer line.
    pub fn line(&mut self, start: (i32, i32), end: (i32, i32), depth: i32, color: [u8; 3]) {
        let steps = (end.0 - start.0).abs().max((end.1 - start.1).abs()).max(1);
        for step in 0..=steps {
            self.pixel(
                start.0 + (end.0 - start.0) * step / steps,
                start.1 + (end.1 - start.1) * step / steps,
                depth,
                color,
            )
        }
    }
    /// Solid voxel box, shaded on its three visible faces.
    pub fn cuboid(&mut self, x: i32, y: i32, width: i32, length: i32, height: i32, color: [u8; 3]) {
        for world_y in y..y + length {
            for world_x in x..x + width {
                let (screen_x, screen_y) = project(world_x, world_y, height);
                self.pixel(
                    screen_x,
                    screen_y,
                    (world_x + world_y) * 8 + height,
                    shade(color, 120),
                );
                self.pixel(
                    screen_x,
                    screen_y + 1,
                    (world_x + world_y) * 8 + height,
                    shade(color, 120),
                );
            }
        }
        for z in 0..height {
            for world_x in x..x + width {
                let (screen_x, screen_y) = project(world_x, y + length, z);
                self.pixel(
                    screen_x,
                    screen_y,
                    (world_x + y + length) * 8 + z,
                    shade(color, 76),
                );
            }
            for world_y in y..y + length {
                let (screen_x, screen_y) = project(x + width, world_y, z);
                self.pixel(screen_x, screen_y, (x + width + world_y) * 8 + z, color);
            }
        }
    }
}
/// Saturating percentage shade.
pub fn shade(color: [u8; 3], percent: u16) -> [u8; 3] {
    color.map(|channel| (u16::from(channel) * percent / 100).min(255) as u8)
}
