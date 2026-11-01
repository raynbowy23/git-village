//! Coastal Earth city composition with cached, depth-tested buildings and periodic motion.
use crate::{
    development::{STAGES, derive},
    layout::{Archetype, Building, language_color, stable_hash},
    mood::{Mood, select},
    motion::{
        FRAME_COUNT, Mode, Surface, TICK_FRAMES, Vehicle, person_pose, traffic, vehicle_body, wave,
    },
    plan::CityPlan,
    setting::Setting,
    snapshot::Snapshot,
    voxel::{Frame, HEIGHT, TRANSPARENT, WIDTH, project, shade},
};
/// Exact warm light used by windows and street lamps, independent of night shading.
pub const WARM_LIGHT: [u8; 3] = [255, 211, 118];
/// Bright firework colors, reserved for Festival bursts and banners.
pub const BURST_COLORS: [[u8; 3]; 3] = [[255, 114, 164], [255, 235, 128], [126, 242, 233]];
/// Human-readable mood label.
pub fn mood_label(mood: Mood) -> &'static str {
    match mood {
        Mood::Abandoned => "ABANDONED",
        Mood::Night => "NIGHT",
        Mood::MarketDay => "MARKET DAY",
        Mood::Festival => "FESTIVAL",
        Mood::Workday => "WORKDAY",
    }
}
/// Format full, unabridged decimal values with thousands separators.
pub fn format_number(number: u64) -> String {
    let digits = number.to_string();
    let mut output = String::new();
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            output.push(',');
        }
        output.push(character);
    }
    output
}
/// Fit all status information in three rows, splitting enormous values instead of clipping them.
pub fn status_lines(snapshot: &Snapshot) -> [String; 3] {
    let development = derive(snapshot);
    let login: String = snapshot.login.chars().take(12).collect();
    let heading = format!(
        "{}  {}  {}",
        login,
        STAGES[development.stage].0,
        mood_label(select(snapshot))
    );
    let commits = format!("{} COMMITS", format_number(snapshot.total_commits));
    let population = format!("{} POP", format_number(development.population));
    let combined = format!("{commits}  {population}");
    if combined.len() <= 62 {
        [heading, combined, String::new()]
    } else {
        [heading, commits, population]
    }
}
fn atmosphere(color: [u8; 3], mood: Mood) -> [u8; 3] {
    if mood == Mood::Night {
        return [
            (u16::from(color[0]) * 2 / 5) as u8,
            (u16::from(color[1]) * 2 / 5) as u8,
            (u16::from(color[2]) * 3 / 5) as u8,
        ];
    }
    if mood == Mood::Abandoned {
        let gray = (u16::from(color[0]) + u16::from(color[1]) + u16::from(color[2])) / 3;
        return color.map(|channel| ((u16::from(channel) + gray * 5) / 6) as u8);
    }
    color
}
/// What surrounds the island.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backdrop {
    /// Sky, sea, far coast, sailboats, and the status strip.
    Scene,
    /// Nothing: the island stands alone on a transparent canvas, like a sticker on the page.
    None,
}
/// Cached static city and depth buffer.
pub struct Renderer {
    backdrop: Backdrop,
    snapshot: Snapshot,
    setting: Setting,
    mood: Mood,
    static_frame: Frame,
    buildings: Vec<Building>,
    plan: CityPlan,
}
impl Renderer {
    /// Render with the full scene around the island.
    pub fn new(snapshot: &Snapshot, setting: Setting) -> Result<Self, String> {
        Self::with_backdrop(snapshot, setting, Backdrop::Scene)
    }
    /// Validate and rasterize the permanent island, buildings and mood decorations.
    pub fn with_backdrop(
        snapshot: &Snapshot,
        setting: Setting,
        backdrop: Backdrop,
    ) -> Result<Self, String> {
        snapshot.validate()?;
        let scene = backdrop == Backdrop::Scene;
        let mood = select(snapshot);
        let development = derive(snapshot);
        let colors = setting.colors();
        let plan = CityPlan::new(snapshot);
        let sky: [u8; 3] = match mood {
            Mood::Night => [29, 42, 71],
            Mood::Abandoned => [154, 169, 173],
            _ => [139, 190, 204],
        };
        let water: [u8; 3] = match mood {
            Mood::Night => [26, 62, 83],
            Mood::Abandoned => [109, 133, 135],
            _ => [54, 121, 145],
        };
        let mut frame = Frame::new(if scene { sky } else { TRANSPARENT });
        for y in (0..HEIGHT as i32).filter(|_| scene) {
            let blend = ((y - 68).clamp(0, 28) * 100 / 28) as u16;
            let color = std::array::from_fn(|channel| {
                ((u16::from(sky[channel]) * (100 - blend) + u16::from(water[channel]) * blend)
                    / 100) as u8
            });
            frame.rect(
                0,
                y,
                WIDTH as i32,
                1,
                -10000,
                shade(color, 100 - ((y - 100).max(0) / 10) as u16),
            );
        }
        for index in (0..14).filter(|_| scene) {
            let x = index * 30 - 10;
            let radius = 14 + (stable_hash(&format!("{}:{index}", snapshot.account_id)) % 8) as i32;
            disc(
                &mut frame,
                x,
                91,
                radius,
                -9500,
                atmosphere(colors[1], mood),
            );
        }
        if scene && mood == Mood::Night {
            disc(&mut frame, 340, 22, 7, -8500, [226, 230, 209]);
            disc(&mut frame, 343, 19, 6, -8400, sky);
        } else if scene && mood != Mood::Abandoned {
            disc(&mut frame, 340, 22, 8, -8500, [248, 225, 161]);
        }
        draw_ground(&mut frame, &plan, setting, mood, development.stage);
        if development.stage >= 3 {
            for road in &plan.roads {
                for (i, (x, y)) in road.tiles().into_iter().enumerate() {
                    if i % 9 < 4 {
                        let (sx, sy) = project(x, y, 0);
                        frame.pixel(sx, sy, (x + y) * 8 + 1, atmosphere([211, 207, 169], mood));
                    }
                }
            }
        }
        if development.stage >= 4 {
            for (x, y) in plan.roads[0].tiles() {
                let (sx, sy) = project(x, y, 0);
                frame.pixel(sx, sy, (x + y) * 8 + 2, atmosphere([160, 170, 166], mood));
            }
        }
        // Bus dwell locations carry a small blue stop sign beside the circuit.
        for vehicle in traffic(&plan, development.stage, snapshot.recent(30))
            .into_iter()
            .filter(|v| v.mode == Mode::Bus)
        {
            let pose = vehicle_body(&plan, vehicle, 0)[0];
            let (sx, sy) = project(pose.x, pose.y, 0);
            let depth = (pose.x + pose.y) * 8 + 8;
            frame.line(
                (sx + 4, sy),
                (sx + 4, sy - 6),
                depth,
                atmosphere([84, 99, 93], mood),
            );
            frame.rect(
                sx + 3,
                sy - 7,
                3,
                2,
                depth + 1,
                atmosphere([70, 133, 178], mood),
            );
        }
        let mut building_list = plan.buildings.clone();
        // Ground shadows are drawn before any walls, with one-pixel dark gaps between lots.
        for building in &building_list {
            for y in building.y..building.y + building.length + 3 {
                for x in building.x + 2..building.x + building.width + 4 {
                    let (sx, sy) = project(x, y, 0);
                    frame.pixel(sx, sy, (x + y) * 8 + 1, atmosphere([65, 99, 80], mood));
                }
            }
        }
        building_list.sort_by_key(|building| building.x + building.y);
        for building in &building_list {
            draw_building(
                &mut frame,
                building,
                setting,
                mood,
                development.stage == 0,
                development.upkeep,
            );
        }
        // Slim elevated line follows the hub's east-west diameter.
        if development.stage >= 5 {
            for t in -28..=28 {
                let (x, y) = (56 + t, 56 - t);
                let (sx, sy) = project(x, y, 21);
                frame.rect(
                    sx,
                    sy,
                    2,
                    1,
                    (x + y) * 8 + 21,
                    atmosphere([137, 155, 151], mood),
                );
                if t.rem_euclid(18) == 0 {
                    frame.cuboid(x, y, 1, 1, 21, atmosphere([130, 143, 140], mood));
                }
            }
        }
        if development.stage >= 2 {
            for tier in 0..(1 + development.longest_streak / 12).min(5) {
                frame.cuboid(
                    52 + tier as i32,
                    52 + tier as i32,
                    8 - tier as i32,
                    8 - tier as i32,
                    6 + tier as i32 * 5,
                    atmosphere([188, 178, 135], mood),
                );
            }
        }
        if mood == Mood::MarketDay {
            for (index, (x, y)) in [(50, 51), (57, 51), (50, 58)].into_iter().enumerate() {
                frame.cuboid(x, y, 4, 3, 3, [164, 115, 74]);
                for stripe in 0..5 {
                    let (sx, sy) = project(x + stripe, y + 3, 6);
                    frame.rect(
                        sx,
                        sy,
                        2,
                        3,
                        (x + y + stripe + 6) * 8 + 8,
                        if stripe % 2 == 0 {
                            [249, 228, 174]
                        } else {
                            [[211, 96, 85], [74, 161, 167], [213, 167, 66]][index]
                        },
                    );
                }
            }
        }
        if mood == Mood::Festival {
            for offset in 0..14 {
                let (sx, sy) = project(49 + offset, 50, 12);
                frame.pixel(sx, sy, 10000, [79, 88, 81]);
                if offset % 4 < 3 {
                    frame.rect(
                        sx,
                        sy + 1,
                        2,
                        3,
                        10001,
                        BURST_COLORS[(offset / 4) as usize % 3],
                    );
                }
            }
        }
        for (index, (x, y)) in [(44, 44), (68, 68), (44, 68), (68, 44)]
            .into_iter()
            .enumerate()
        {
            let (sx, sy) = project(x, y, 0);
            let depth = (x + y) * 8 + 12;
            frame.line(
                (sx, sy),
                (sx, sy - 8),
                depth,
                atmosphere([84, 99, 93], mood),
            );
            let lit = mood != Mood::Abandoned && (index == 0 || index <= development.upkeep);
            frame.rect(
                sx - 1,
                sy - 9,
                3,
                2,
                depth + 1,
                if lit { WARM_LIGHT } else { [102, 115, 113] },
            );
        }
        for index in 0..28 {
            let x = (index * 37 + snapshot.account_id as i32 % 13).rem_euclid(110);
            let y = (index * 53 + 17).rem_euclid(110);
            if plan.land.contains(&(x, y))
                && plan.surface(x, y) == Surface::Grass
                && !building_list
                    .iter()
                    .any(|building| (building.x - x).abs() < 8 && (building.y - y).abs() < 8)
            {
                prop(&mut frame, x, y, setting, 0, mood);
            }
        }
        if mood == Mood::Abandoned {
            for index in 0..100 {
                let x = (index * 37 + 11) % 111;
                let y = (index * 47 + 19) % 111;
                if !plan.land.contains(&(x, y)) {
                    continue;
                }
                let (sx, sy) = project(x, y, 0);
                let depth = (x + y) * 8 + 2;
                frame.line((sx, sy), (sx + 1, sy - 2), depth, [112, 132, 104]);
            }
        }
        if scene {
            frame.rect(0, 234, WIDTH as i32, 22, 100000, [23, 30, 42]);
        }
        for (index, line) in status_lines(snapshot).iter().enumerate().filter(|_| scene) {
            text(&mut frame, 4, 236 + index as i32 * 7, line, [224, 224, 203]);
        }
        if scene && status_lines(snapshot)[2].is_empty() {
            let mut languages: Vec<_> = snapshot.languages.iter().collect();
            languages.sort_by_key(|language| (std::cmp::Reverse(language.commits), &language.name));
            for (index, language) in languages.into_iter().take(5).enumerate() {
                frame.rect(
                    4 + index as i32 * 6,
                    251,
                    4,
                    3,
                    100003,
                    language_color(&language.name, language.color.as_deref()),
                );
            }
        }
        Ok(Self {
            backdrop,
            snapshot: snapshot.clone(),
            setting,
            mood,
            static_frame: frame,
            buildings: building_list,
            plan,
        })
    }
    /// First loop frame.
    pub fn still(&self) -> Frame {
        self.frame(0)
    }
    /// Visible traffic allocation, including the Abandoned suppression rule.
    pub fn vehicles(&self) -> Vec<Vehicle> {
        if self.mood == Mood::Abandoned {
            Vec::new()
        } else {
            traffic(
                &self.plan,
                derive(&self.snapshot).stage,
                self.snapshot.recent(30),
            )
        }
    }
    /// Number of active construction sites, bounded by recent weekly work and city stage.
    pub fn crane_count(&self) -> usize {
        crane_count(self.snapshot.recent(7), derive(&self.snapshot).stage)
    }
    /// Compose a pure, exactly periodic dynamic frame.
    pub fn frame(&self, index: usize) -> Frame {
        let index = (index % FRAME_COUNT + 11) % FRAME_COUNT;
        let route_frame =
            (index + (self.snapshot.account_id % FRAME_COUNT as u64) as usize) % FRAME_COUNT;
        let mut frame = self.static_frame.clone();
        let development = derive(&self.snapshot);
        let population = (development.population.saturating_add(1).ilog2() as usize * 3).min(46);
        let population = match self.mood {
            Mood::Abandoned => 1,
            Mood::Night => 3,
            Mood::Festival => population + 14,
            Mood::MarketDay => population + 8,
            _ => population,
        };
        for person in 0..population {
            let district_person = if person
                >= population.saturating_sub(match self.mood {
                    Mood::Festival => 14,
                    Mood::MarketDay => 8,
                    _ => 0,
                }) {
                person * self.plan.walks.len()
            } else {
                person
            };
            let pose = person_pose(&self.plan, district_person, route_frame);
            let (sx, sy) = project(
                pose.x,
                pose.y,
                3 + ((index / TICK_FRAMES + person) % 4 / 2) as i32,
            );
            let depth = (pose.x + pose.y) * 8 + 4;
            frame.pixel(sx, sy, depth, [236, 197, 146]);
            frame.rect(
                sx,
                sy + 1,
                1,
                2,
                depth,
                [73 + (person % 4) as u8 * 39, 92, 132],
            );
        }
        for vehicle in self.vehicles() {
            let mode = vehicle.mode;
            draw_vehicle(
                &mut frame,
                &vehicle_body(&self.plan, vehicle, route_frame),
                mode,
                self.setting.materials().vehicles[mode as usize],
            );
        }
        for (building_index, building) in self.buildings.iter().enumerate() {
            if development.stage == 0 || self.mood == Mood::Abandoned {
                continue;
            }
            let (sx, sy) = project(building.x + 1, building.y + 1, building.height + 6);
            let depth = (building.x + building.y + 8) * 8 + building.height + 8;
            match building.archetype {
                Archetype::Foundry => {
                    for puff in 0..if self.buildings.len() > 80 { 1 } else { 3 } {
                        let phase = (index / TICK_FRAMES + puff * 32 + building_index * 7) % 96;
                        let rise = phase as i32 / 10;
                        frame.rect(
                            sx + wave(index, building_index * 9, 2),
                            sy - rise,
                            2 + rise / 4,
                            2,
                            depth,
                            atmosphere([191, 196, 183], self.mood),
                        );
                    }
                }
                Archetype::Tower => {
                    if (index / TICK_FRAMES + building_index * 8) % 48 < 24 {
                        frame.pixel(sx, sy, depth, WARM_LIGHT);
                    }
                }
                Archetype::Market => {
                    frame.rect(
                        sx,
                        sy + 9,
                        3,
                        1,
                        depth,
                        if (index / TICK_FRAMES + building_index * 9) % 48 < 24 {
                            [249, 209, 107]
                        } else {
                            [125, 87, 80]
                        },
                    );
                }
                Archetype::Temple => {
                    frame.line(
                        (sx, sy),
                        (sx + 3, sy + wave(index, building_index * 3, 1)),
                        depth,
                        [229, 142, 105],
                    );
                }
                Archetype::Academy => {
                    frame.pixel(
                        sx + wave(index, building_index * 5, 2),
                        sy + 2,
                        depth,
                        [66, 77, 100],
                    );
                }
                Archetype::Depot => {}
            }
            if building_index < self.crane_count() {
                let (base_x, base_y) = project(
                    building.x + building.width,
                    building.y + building.length / 2,
                    0,
                );
                let top = base_y - building.height - 4;
                let crane_depth = (building.x + building.y + building.width + building.length) * 8
                    + building.height
                    + 6;
                frame.line((base_x, base_y), (base_x, top), crane_depth, [228, 180, 77]);
                frame.line(
                    (base_x - 3, top),
                    (base_x + 6, top + wave(index, building_index * 13, 1)),
                    crane_depth,
                    [228, 180, 77],
                );
                let hook = top + 3 + wave(index, building_index * 17, 1);
                frame.line(
                    (base_x + 5, top),
                    (base_x + 5, hook),
                    crane_depth,
                    [126, 110, 78],
                );
                frame.rect(base_x + 4, hook, 2, 2, crane_depth, [188, 160, 117]);
            }
        }
        self.nature(&mut frame, index);
        if self.mood == Mood::Festival {
            for (burst, burst_color) in BURST_COLORS.iter().enumerate() {
                let phase = (index * 2 / TICK_FRAMES + burst * 32) % 96;
                let radius = 2 + phase as i32 / 5;
                let center = if self.backdrop == Backdrop::None {
                    let bounds = self.city_bounds();
                    let x =
                        bounds.0 + 22 + (bounds.2 - bounds.0 - 44).max(0) * (burst as i32 + 1) / 4;
                    (x, (bounds.1 - 3).max(24))
                } else {
                    (40 + burst as i32 * 80, 24 + (burst as i32 % 2) * 10)
                };
                let color = if phase < 64 {
                    *burst_color
                } else {
                    std::array::from_fn(|channel| {
                        ((u16::from(burst_color[channel]) * (96 - phase) as u16
                            + u16::from([139_u8, 190, 204][channel]) * (phase - 64) as u16)
                            / 32) as u8
                    })
                };
                if phase < 90 {
                    for (dx, dy) in [
                        (1, 0),
                        (1, 1),
                        (0, 1),
                        (-1, 1),
                        (-1, 0),
                        (-1, -1),
                        (0, -1),
                        (1, -1),
                    ] {
                        let end = (center.0 + dx * radius, center.1 + dy * radius);
                        frame.line(
                            (
                                center.0 + dx * (radius - 6).max(0),
                                center.1 + dy * (radius - 6).max(0),
                            ),
                            end,
                            20000,
                            color,
                        );
                        frame.rect(end.0, end.1, 2, 2, 20000, color);
                    }
                }
            }
        }
        frame
    }
    fn city_bounds(&self) -> (i32, i32, i32, i32) {
        let mut bounds = (WIDTH as i32, HEIGHT as i32, 0, 0);
        for (i, color) in self.static_frame.pixels.iter().enumerate() {
            if *color != TRANSPARENT {
                let (x, y) = ((i % WIDTH) as i32, (i / WIDTH) as i32);
                bounds.0 = bounds.0.min(x);
                bounds.1 = bounds.1.min(y);
                bounds.2 = bounds.2.max(x);
                bounds.3 = bounds.3.max(y);
            }
        }
        bounds
    }
    fn nature(&self, frame: &mut Frame, index: usize) {
        let scene = self.backdrop == Backdrop::Scene;
        for cloud in (0..8).filter(|_| scene) {
            let x = cloud * 55 - 12 + wave(index, cloud as usize * 13, 8);
            let y = 15 + (cloud % 3) * 12;
            let color = match self.mood {
                Mood::Night => [62, 80, 105],
                Mood::Abandoned => [181, 190, 190],
                _ => [223, 235, 226],
            };
            disc(frame, x, y, 5, -6000, color);
            frame.rect(x - 8, y, 24, 4, -6000, color);
        }
        if scene && self.mood == Mood::Night {
            for star in 0..23 {
                let x = 9 + (star * 37) % 368;
                let y = 5 + (star * 19) % 58;
                let color = if (index / TICK_FRAMES + star as usize * 7) % 48 < 24 {
                    [226, 230, 209]
                } else {
                    [112, 143, 166]
                };
                frame.pixel(x, y, -5500, color);
            }
        }
        for glint in (0..32).filter(|_| scene) {
            let x = 4 + (glint * 37) % 376 + wave(index, glint as usize * 7, 2);
            let y = 124 + (glint * 13) % 104;
            let color = if (index / TICK_FRAMES + glint as usize * 3) % 48 < 24 {
                [126, 180, 185]
            } else {
                [80, 145, 159]
            };
            frame.rect(x, y, 3, 1, -7000, atmosphere(color, self.mood));
        }
        for boat in (0..2).filter(|_| scene) {
            let x = 22 + boat * 337 + wave(index, boat as usize * 32, 3);
            let y = 140 + boat * 8 + wave(index, boat as usize * 24, 1);
            frame.line((x - 5, y), (x + 4, y), -5000, [119, 81, 57]);
            frame.line((x - 3, y + 1), (x + 2, y + 1), -5000, [100, 73, 57]);
            frame.line((x, y), (x, y - 12), -5000, [185, 181, 153]);
            for row in 0..9 {
                frame.rect(
                    x - row / 2,
                    y - 11 + row,
                    row / 2 + 1,
                    1,
                    -5000,
                    atmosphere([240, 227, 184], self.mood),
                );
            }
        }
        for bird in (0..5).filter(|_| scene) {
            let x = 26 + bird * 75 + wave(index, bird as usize * 9, 8);
            let y = 54 + wave(index, bird as usize * 11, 2);
            frame.line((x - 2, y), (x, y + 1), 4000, [53, 74, 90]);
            frame.line(
                (x, y + 1),
                (x + 2, y + wave(index, 0, 1)),
                4000,
                [53, 74, 90],
            );
        }
        for tree in 0..12 {
            let x = (tree * 37 + 7) % 110;
            let y = (tree * 53 + 19) % 110;
            if self.plan.land.contains(&(x, y))
                && self.plan.surface(x, y) == Surface::Grass
                && !self
                    .buildings
                    .iter()
                    .any(|building| (building.x - x).abs() < 8 && (building.y - y).abs() < 8)
            {
                prop(
                    frame,
                    x,
                    y,
                    self.setting,
                    wave(index, tree as usize * 7, 1),
                    self.mood,
                );
            }
        }
    }
}
fn disc(frame: &mut Frame, x: i32, y: i32, radius: i32, depth: i32, color: [u8; 3]) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= radius * radius {
                frame.pixel(x + dx, y + dy, depth, color);
            }
        }
    }
}
fn prop(frame: &mut Frame, x: i32, y: i32, setting: Setting, sway: i32, mood: Mood) {
    let (sx, sy) = project(x, y, 0);
    let depth = (x + y) * 8 + 8;
    frame.rect(
        sx,
        sy - 5,
        1,
        6,
        depth,
        atmosphere(setting.materials().trunk, mood),
    );
    disc(
        frame,
        sx + sway,
        sy - 8,
        3,
        depth,
        atmosphere(setting.materials().foliage, mood),
    );
    disc(
        frame,
        sx + sway - 1,
        sy - 9,
        2,
        depth + 1,
        atmosphere(shade(setting.materials().foliage, 130), mood),
    );
}
fn draw_building(
    frame: &mut Frame,
    building: &Building,
    setting: Setting,
    mood: Mood,
    outpost: bool,
    upkeep: usize,
) {
    let Building {
        x,
        y,
        height,
        width,
        length,
        archetype,
        color,
        ..
    } = *building;
    let tone = 90 + (stable_hash(&format!("{x}:{y}")) % 19) as u16;
    let color = atmosphere(
        shade(
            color.map(|channel| ((u16::from(channel) * 3 + 190) / 4) as u8),
            tone,
        ),
        mood,
    );
    if outpost {
        let (sx, sy) = project(x + 2, y + 2, 0);
        for row in 0..9 {
            frame.rect(
                sx - row / 2,
                sy - 8 + row,
                1 + row,
                1,
                (x + y + 10) * 8,
                [220, 190, 128],
            );
        }
        frame.rect(sx, sy - 3, 2, 4, (x + y + 10) * 8 + 1, [83, 69, 55]);
        let (fx, fy) = project(x + 7, y + 3, 0);
        frame.rect(fx, fy - 3, 2, 3, (x + y + 15) * 8, [246, 177, 65]);
        return;
    }
    frame.cuboid(x, y, width, length, height, color);
    let depth = (x + y + width + length + 2) * 8 + height;
    let (sx, sy) = project(x + width / 2, y + length / 2, height);
    let roof = atmosphere(setting.materials().roof, mood);
    for edge in 0..width {
        let (rx, ry) = project(x + edge, y + length, height);
        frame.pixel(rx, ry, (x + y + edge + length) * 8 + height + 1, roof);
    }
    match archetype {
        Archetype::Foundry => {
            for tooth in (0..width).step_by(2) {
                frame.cuboid(x + tooth, y, 1, length, height + 2, color);
            }
            frame.cuboid(
                x + 1,
                y + 1,
                1,
                1,
                height + 3,
                atmosphere([102, 99, 89], mood),
            );
        }
        Archetype::Academy => {
            disc(
                frame,
                sx,
                sy - if width < 6 { 1 } else { 2 },
                if width < 6 { 1 } else { 2 },
                depth,
                shade(color, 125),
            );
            frame.rect(sx - 2, sy, 5, 1, depth, shade(color, 105));
        }
        Archetype::Market => {
            for stripe in 0..width {
                let (ax, ay) = project(x + stripe, y + length, height - 2);
                frame.rect(
                    ax,
                    ay,
                    1,
                    3,
                    depth,
                    if stripe % 2 == 0 {
                        [235, 216, 161]
                    } else {
                        roof
                    },
                );
            }
        }
        Archetype::Tower => {
            frame.line(
                (sx, sy),
                (sx, sy - if width < 6 { 3 } else { 5 }),
                depth,
                atmosphere([151, 163, 164], mood),
            );
        }
        Archetype::Depot => {
            let (dx, dy) = project(x + width, y + length / 2, 0);
            frame.rect(dx, dy - 4, 2, 4, depth, atmosphere([75, 79, 78], mood));
        }
        Archetype::Temple => {
            for tier in 0..2 {
                frame.cuboid(
                    x + tier,
                    y + tier,
                    (width - tier).max(1),
                    (length - tier).max(1),
                    height + tier * if width < 6 { 1 } else { 3 },
                    shade(roof, 100 + tier as u16 * 12),
                );
            }
            frame.line(
                (sx, sy - 3),
                (sx, sy - if width < 6 { 3 } else { 7 }),
                depth,
                [202, 175, 112],
            );
        }
    }
    // Windows are applied after all material shading, so night lighting stays warm.
    for row in (3..height - 1).step_by(5) {
        for column in (1..width).step_by(2) {
            let lit = (row as usize + column as usize * 3 + x.rem_euclid(1000) as usize) % 11
                <= if mood == Mood::Night {
                    upkeep.min(7) / 3
                } else {
                    upkeep.min(7)
                };
            let light = if mood == Mood::Abandoned || !lit {
                [48, 66, 76]
            } else if mood == Mood::Night {
                WARM_LIGHT
            } else {
                [225, 215, 156]
            };
            for (wx, wy) in [
                (x + column, y + length),
                (x + width, y + column.min(length - 1)),
            ] {
                let (px, py) = project(wx, wy, row);
                frame.pixel(px, py, (wx + wy) * 8 + row + 2, light);
            }
        }
    }
    if mood == Mood::Abandoned {
        for row in 0..height.min(15) {
            if row % 3 != 1 {
                let (vx, vy) = project(x + width, y + 1, row);
                frame.rect(
                    vx,
                    vy,
                    2,
                    1,
                    (x + y + width + 1) * 8 + row + 3,
                    [112, 132, 104],
                );
            }
        }
    }
}
/// Compact 3-by-5 status font; unsupported glyphs use a small box.
pub fn text(frame: &mut Frame, x: i32, y: i32, value: &str, color: [u8; 3]) {
    for (index, character) in value.to_ascii_uppercase().chars().enumerate() {
        if x + index as i32 * 4 > WIDTH as i32 - 5 {
            break;
        }
        let bits = match character {
            'A' => 0b010101111101101,
            'B' => 0b110101110101110,
            'C' => 0b011100100100011,
            'D' => 0b110101101101110,
            'E' => 0b111100110100111,
            'F' => 0b111100110100100,
            'G' => 0b011100101101011,
            'H' => 0b101101111101101,
            'I' => 0b111010010010111,
            'J' => 0b001001001101010,
            'K' => 0b101101110101101,
            'L' => 0b100100100100111,
            'M' => 0b101111111101101,
            'N' => 0b101111111111101,
            'O' => 0b010101101101010,
            'P' => 0b110101110100100,
            'Q' => 0b010101101111011,
            'R' => 0b110101110101101,
            'S' => 0b011100010001110,
            'T' => 0b111010010010010,
            'U' => 0b101101101101111,
            'V' => 0b101101101101010,
            'W' => 0b101101111111101,
            'X' => 0b101101010101101,
            'Y' => 0b101101010010010,
            'Z' => 0b111001010100111,
            '0' => 0b111101101101111,
            '1' => 0b010110010010111,
            '2' => 0b110001010100111,
            '3' => 0b110001010001110,
            '4' => 0b101101111001001,
            '5' => 0b111100110001110,
            '6' => 0b011100111101111,
            '7' => 0b111001010010010,
            '8' => 0b111101111101111,
            '9' => 0b111101111001110,
            ',' => 0b000000000010100,
            ' ' => 0,
            '-' => 0b000000111000000,
            _ => 0b111101101101111,
        };
        for row in 0..5 {
            for column in 0..3 {
                if bits & (1 << (14 - row * 3 - column)) != 0 {
                    frame.pixel(x + index as i32 * 4 + column, y + row, 100002, color)
                }
            }
        }
    }
}
fn draw_vehicle(frame: &mut Frame, sections: &[crate::motion::Pose], mode: Mode, color: [u8; 3]) {
    let height = if mode == Mode::Bicycle { 2 } else { 3 };
    let length = sections.len();
    for (section, body) in sections.iter().enumerate() {
        let (vx, vy, z) = (body.x, body.y, body.z);
        if section % 5 == 4 && matches!(mode, Mode::Tram | Mode::Train) {
            continue;
        }
        let (sx, sy) = project(vx, vy, z + height);
        let depth = (vx + vy) * 8 + z + height + 2;
        frame.rect(
            sx - 1,
            sy,
            3,
            height,
            depth,
            if section % 5 == 3 {
                [62, 92, 107]
            } else {
                color
            },
        );
        frame.pixel(sx, sy, depth + 1, shade(color, 125));
        if section == 1 || section == length - 2 {
            frame.pixel(sx, sy + height, depth, [33, 43, 47]);
        }
    }
}

/// Rasterize bare terrain and soil for coast and slab verification, without props or buildings.
pub fn bare_ground(plan: &CityPlan, setting: Setting) -> Frame {
    let mut frame = Frame::new(TRANSPARENT);
    draw_ground(&mut frame, plan, setting, Mood::Workday, 3);
    frame
}
fn draw_ground(frame: &mut Frame, plan: &CityPlan, setting: Setting, mood: Mood, stage: usize) {
    let colors = setting.colors();
    // Every exposed front edge gets its own depth-tested earth face, including concave coasts.
    for &(x, y) in &plan.land {
        // Solid tile columns also cover interior samples that can be visible through a concave projected coast.
        for below in 0..=9 {
            let (sx, sy) = project(x, y, -below);
            let soil = atmosphere(
                if below < 5 {
                    [150, 116, 80]
                } else {
                    [116, 90, 66]
                },
                mood,
            );
            frame.pixel(sx, sy, (x + y) * 8 - below, soil);
            frame.pixel(sx, sy + 1, (x + y) * 8 - below, soil);
        }
    }
    for &(x, y) in &plan.land {
        for (dx, dy, tone) in [(1, 0, 100), (0, 1, 76)] {
            if !plan.land.contains(&(x + dx, y + dy)) {
                // Rasterize the entire tile edge, including its shared corner. A single sample per tile leaves alternating empty screen columns.
                for below in 0..=9 {
                    let soil = if below < 5 {
                        [150, 116, 80]
                    } else {
                        [116, 90, 66]
                    };
                    for along in 0..=1 {
                        let (wx, wy) = (x + dx + dy * along, y + dy + dx * along);
                        let (sx, sy) = project(wx, wy, -below);
                        frame.pixel(
                            sx,
                            sy,
                            (wx + wy) * 8 - below,
                            atmosphere(shade(soil, tone), mood),
                        );
                        frame.pixel(
                            sx,
                            sy + 1,
                            (wx + wy) * 8 - below,
                            atmosphere(shade(soil, tone), mood),
                        );
                    }
                }
            }
        }
    }
    for &(x, y) in &plan.land {
        let coast = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|&(dx, dy)| !plan.land.contains(&(x + dx, y + dy)));
        let color = if coast {
            [213, 198, 150]
        } else {
            match plan.surface(x, y) {
                Surface::Road => {
                    if stage < 2 {
                        [158, 141, 103]
                    } else {
                        colors[3]
                    }
                }
                Surface::Sidewalk => setting.materials().sidewalk,
                Surface::Grass => shade(
                    colors[2],
                    if (x * 7 + y * 3).rem_euclid(19) == 0 {
                        108
                    } else {
                        100
                    },
                ),
            }
        };
        let (sx, sy) = project(x, y, 0);
        let color = atmosphere(color, mood);
        frame.pixel(sx, sy, (x + y) * 8, color);
        frame.pixel(sx, sy + 1, (x + y) * 8, color);
    }
}

/// Weekly construction demand: a small amount of work supports one proportionate crane.
pub fn crane_count(recent: u64, stage: usize) -> usize {
    match recent {
        0 => 0,
        1..=9 => 1,
        10..=49 => 2,
        50..=499 => 3,
        _ => {
            if stage >= 4 {
                4
            } else {
                3
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_fireworks_stay_over_the_city_and_close_to_its_skyline() {
        let snapshot =
            Snapshot::from_json(include_bytes!("../tests/fixtures/festival.json")).unwrap();
        let renderer = Renderer::with_backdrop(&snapshot, Setting::Earth, Backdrop::None).unwrap();
        assert_eq!(renderer.mood, Mood::Festival);
        let bounds = renderer.city_bounds();
        let mut seen = false;
        for index in 0..FRAME_COUNT {
            let frame = renderer.frame(index);
            for (i, color) in frame
                .pixels
                .iter()
                .enumerate()
                .filter(|(_, color)| BURST_COLORS.contains(color))
            {
                let (x, y) = ((i % WIDTH) as i32, (i / WIDTH) as i32);
                assert!(
                    (bounds.0..=bounds.2).contains(&x),
                    "burst outside the city's width: {x}"
                );
                assert!(y >= bounds.1 - 24, "burst stranded above the skyline: {y}");
                let _ = color;
                seen = true;
            }
        }
        assert!(seen);
    }
}
