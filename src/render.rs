//! Coastal Earth city composition with cached, depth-tested buildings and periodic motion.
use crate::{
    development::{STAGES, derive},
    layout::{Archetype, Building, language_color, stable_hash},
    mood::{Mood, select},
    motion::{
        FRAME_COUNT, Mode, Surface, TICK_FRAMES, Vehicle, person_pose, traffic, vehicle_body, wave,
    },
    plan::CityPlan,
    setting::{Celestial, Flora, Setting},
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
    detail: f64,
}
impl Renderer {
    /// Keep this share of walkers, vehicles and animated buildings, so a loop can be made to fit the GIF budget. The city itself does not change.
    pub fn with_detail(mut self, detail: f64) -> Self {
        self.detail = detail;
        self
    }
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
        let materials = setting.materials();
        let tone = match mood {
            Mood::Night => 1,
            Mood::Abandoned => 2,
            _ => 0,
        };
        let sky = materials.sky[tone];
        let water = materials.water[tone];
        let mut frame = city_canvas(&plan, scene, if scene { sky } else { TRANSPARENT });
        let (left, top) = (frame.left, frame.top);
        let (canvas_width, canvas_height) = (frame.width as i32, frame.height as i32);
        for row in (0..canvas_height).filter(|_| scene) {
            let y = row;
            let blend = ((y - 68).clamp(0, 28) * 100 / 28) as u16;
            let color = std::array::from_fn(|channel| {
                ((u16::from(sky[channel]) * (100 - blend) + u16::from(water[channel]) * blend)
                    / 100) as u8
            });
            frame.rect(
                left,
                top + y,
                canvas_width,
                1,
                -10000,
                shade(color, 100 - ((y - 100).max(0) / 10).min(50) as u16),
            );
        }
        for index in (0..canvas_width / 30 + 2).filter(|_| scene) {
            let x = left + index * 30 - 10;
            let radius = 14 + (stable_hash(&format!("{}:{index}", snapshot.account_id)) % 8) as i32;
            disc(
                &mut frame,
                x,
                top + 91,
                radius,
                -9500,
                atmosphere(colors[1], mood),
            );
        }
        if scene && materials.celestial == Celestial::TwinMoons && mood != Mood::Abandoned {
            let moon = left + canvas_width - 60;
            disc(&mut frame, moon, top + 18, 4, -8500, [236, 214, 186]);
            disc(&mut frame, moon + 30, top + 34, 2, -8500, [214, 196, 176]);
        } else if materials.celestial != Celestial::Sun {
            // Nothing else overhead in this setting.
        } else if scene && mood == Mood::Night {
            let moon = left + canvas_width - 44;
            disc(&mut frame, moon, top + 22, 7, -8500, [226, 230, 209]);
            disc(&mut frame, moon + 3, top + 19, 6, -8400, sky);
        } else if scene && mood != Mood::Abandoned {
            let sun = left + canvas_width - 44;
            disc(&mut frame, sun, top + 22, 8, -8500, [248, 225, 161]);
        }
        if scene {
            backdrop_details(&mut frame, setting, mood, snapshot.account_id);
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
        for building in &mut building_list {
            building.color = setting.tinted(building.color);
        }
        // Ground shadows are drawn before any walls, with one-pixel dark gaps between lots.
        for building in &building_list {
            for y in building.y..building.y + building.length + 3 {
                for x in building.x + 2..building.x + building.width + 4 {
                    let (sx, sy) = project(x, y, 0);
                    frame.pixel(sx, sy, (x + y) * 8 + 1, atmosphere(materials.shadow, mood));
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
        // Grove trees hold still, because animating them would grow the GIF.
        for &(x, y) in &plan.trees {
            if materials.flora == Flora::Trees && (x / 4 + y / 4).rem_euclid(3) == 0 {
                pine(&mut frame, x, y, setting, mood);
            } else {
                prop(&mut frame, x, y, setting, 0, mood);
            }
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
            for &(x, y) in &plan.land {
                if (x * 37 + y * 47).rem_euclid(97) != 0 {
                    continue;
                }
                let (sx, sy) = project(x, y, 0);
                let depth = (x + y) * 8 + 2;
                frame.line((sx, sy), (sx + 1, sy - 2), depth, [112, 132, 104]);
            }
        }
        let strip = top + canvas_height - 22;
        if scene {
            frame.rect(left, strip, canvas_width, 22, 100000, [23, 30, 42]);
        }
        for (index, line) in status_lines(snapshot).iter().enumerate().filter(|_| scene) {
            text(
                &mut frame,
                left + 4,
                strip + 2 + index as i32 * 7,
                line,
                [224, 224, 203],
            );
        }
        if scene && status_lines(snapshot)[2].is_empty() {
            let mut languages: Vec<_> = snapshot.languages.iter().collect();
            languages.sort_by_key(|language| (std::cmp::Reverse(language.commits), &language.name));
            for (index, language) in languages.into_iter().take(5).enumerate() {
                frame.rect(
                    left + 4 + index as i32 * 6,
                    strip + 17,
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
            detail: 1.0,
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
        // Walkers follow yearly commits and the size of the city, with no ceiling.
        let population = development.population.saturating_add(1).ilog2() as usize * 3
            + crate::motion::crowd(self.buildings.len());
        let thin = |count: usize| (count as f64 * self.detail).ceil() as usize;
        let gathered = thin(match self.mood {
            Mood::Festival => 14,
            Mood::MarketDay => 8,
            _ => 0,
        });
        let population = match self.mood {
            Mood::Abandoned => thin(1),
            Mood::Night => thin(3),
            _ => thin(population) + gathered,
        };
        for person in 0..population {
            let district_person = if person >= population.saturating_sub(gathered) {
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
        let mut vehicles = self.vehicles();
        vehicles.truncate(thin(vehicles.len()));
        for vehicle in vehicles {
            let mode = vehicle.mode;
            draw_vehicle(
                &mut frame,
                &vehicle_body(&self.plan, vehicle, route_frame),
                mode,
                self.setting.materials().vehicles[mode as usize],
                self.setting.materials().lift,
            );
        }
        // Past this many, only an even sample of buildings animates, because each one adds to every frame of the GIF.
        let stride =
            (((self.buildings.len() / 160 + 1) as f64 / self.detail).ceil() as usize).max(1);
        for (building_index, building) in self.buildings.iter().enumerate() {
            if development.stage == 0
                || self.mood == Mood::Abandoned
                || (building_index % stride != 0 && building_index >= self.crane_count())
            {
                continue;
            }
            let (sx, sy) = project(building.x + 1, building.y + 1, building.height + 6);
            let depth = (building.x + building.y + 8) * 8 + building.height + 8;
            match building.archetype {
                Archetype::Foundry => {
                    for puff in 0..if self.buildings.len() > 80 || self.detail < 1.0 {
                        1
                    } else {
                        3
                    } {
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
                    (x, (bounds.1 - 3).max(frame.top + 24))
                } else {
                    (
                        frame.left + 40 + burst as i32 * 80,
                        frame.top + 24 + (burst as i32 % 2) * 10,
                    )
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
    /// A few small moving things that belong to the setting, placed among its open ground.
    fn ambient(&self, frame: &mut Frame, index: usize) {
        if self.setting == Setting::Earth {
            return;
        }
        let trees = &self.plan.trees;
        let wanted = ((5.0 * self.detail).ceil() as usize).min(trees.len());
        let spots: Vec<_> = trees
            .iter()
            .step_by((trees.len() / wanted.max(1)).max(1))
            .take(wanted)
            .copied()
            .collect();
        let tick = index / TICK_FRAMES;
        for (item, &(x, y)) in spots.iter().enumerate() {
            let (sx, sy) = project(x, y, 0);
            let depth = (x + y) * 8 + 10;
            let phase = (tick + item * 19) % 96;
            let mood = self.mood;
            match self.setting {
                Setting::Mars => {
                    // A dust devil twists as it climbs.
                    for level in 0..6 {
                        let drift = wave(index, item * 11 + level as usize * 7, 2);
                        frame.rect(
                            sx + drift - level / 2,
                            sy - level * 2,
                            2 + level,
                            2,
                            depth,
                            atmosphere([222, 170, 120], mood),
                        );
                    }
                }
                Setting::Sky => {
                    // A thin stream spills over the edge beneath this spot's side of the island.
                    let reach = self
                        .plan
                        .land
                        .iter()
                        .filter(|&&(lx, ly)| lx - ly == x - y)
                        .map(|&(lx, ly)| lx + ly)
                        .max()
                        .unwrap_or(x + y);
                    let (fx, fy) = project((reach + x - y) / 2, (reach - x + y) / 2, 0);
                    for drop in 0..4 {
                        let fall = ((phase + drop * 24) % 96) as i32 / 6;
                        frame.rect(
                            fx,
                            fy + 10 + fall,
                            1,
                            2,
                            reach * 8 + 40,
                            atmosphere([236, 246, 250], mood),
                        );
                    }
                }
                Setting::Historic => {
                    // A flag on a pole, and a sheep grazing below it.
                    frame.line((sx + 4, sy), (sx + 4, sy - 12), depth, [92, 70, 48]);
                    frame.line(
                        (sx + 5, sy - 12),
                        (sx + 9, sy - 12 + wave(index, item * 13, 1)),
                        depth,
                        atmosphere([190, 70, 58], mood),
                    );
                    let grazing = wave(index, item * 29, 1);
                    frame.rect(
                        sx - 6 + grazing,
                        sy - 2,
                        4,
                        2,
                        depth,
                        atmosphere([232, 228, 214], mood),
                    );
                    frame.pixel(
                        sx - 7 + grazing,
                        sy - 2,
                        depth,
                        atmosphere([62, 54, 50], mood),
                    );
                }
                Setting::Futuristic => {
                    // A hologram flickers above the pylon, and a light drifts past it.
                    if phase < 64 {
                        frame.rect(
                            sx - 2,
                            sy - 20,
                            5,
                            3,
                            depth,
                            atmosphere([240, 110, 210], mood),
                        );
                    }
                    frame.pixel(
                        sx + wave(index, item * 17, 12),
                        sy - 24 + wave(index, item * 7, 3),
                        depth,
                        atmosphere([150, 240, 250], mood),
                    );
                }
                Setting::Underwater => {
                    // Bubbles rise from the kelp, and a small school crosses it.
                    for bubble in 0..3 {
                        let rise = ((phase + bubble * 32) % 96) as i32 / 3;
                        frame.pixel(
                            sx + 1,
                            sy - 12 - rise,
                            depth,
                            atmosphere([190, 232, 240], mood),
                        );
                    }
                    for fish in 0..4 {
                        frame.rect(
                            sx + wave(index, item * 23, 14) + fish * 4,
                            sy - 18 + (fish * 5) % 4,
                            2,
                            1,
                            depth,
                            atmosphere(self.setting.materials().flier, mood),
                        );
                    }
                }
                Setting::Underground => {
                    // A crystal glints, and a drop falls toward it.
                    if phase < 24 {
                        frame.pixel(sx + 1, sy - 8, depth + 1, [226, 214, 255]);
                    }
                    let fall = phase as i32 / 3;
                    frame.pixel(
                        sx + 3,
                        sy - 34 + fall,
                        depth,
                        atmosphere([150, 190, 220], mood),
                    );
                }
                Setting::Earth => {}
            }
        }
    }
    fn city_bounds(&self) -> (i32, i32, i32, i32) {
        let canvas = &self.static_frame;
        let mut bounds = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (i, color) in canvas.pixels.iter().enumerate() {
            if *color != TRANSPARENT {
                let x = canvas.left + (i % canvas.width) as i32;
                let y = canvas.top + (i / canvas.width) as i32;
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
        let (left, top) = (frame.left, frame.top);
        let canvas_width = frame.width as i32;
        // The base scene repeats across a canvas wider or taller than it.
        let across: Vec<i32> = (0..(canvas_width + 383) / 384)
            .filter(|_| scene)
            .map(|tile| left + tile * 384)
            .collect();
        let down: Vec<i32> = (0..(frame.height as i32 - 124 + 103).max(0) / 104)
            .map(|tile| top + tile * 104)
            .collect();
        for &offset in &across {
            for cloud in 0..8 {
                let x = offset + cloud * 55 - 12 + wave(index, cloud as usize * 13, 8);
                let y = top + 15 + (cloud % 3) * 12;
                let color = if self.setting != Setting::Earth {
                    atmosphere(self.setting.materials().cloud, self.mood)
                } else {
                    match self.mood {
                        Mood::Night => [62, 80, 105],
                        Mood::Abandoned => [181, 190, 190],
                        _ => [223, 235, 226],
                    }
                };
                disc(frame, x, y, 5, -6000, color);
                frame.rect(x - 8, y, 24, 4, -6000, color);
            }
            if self.mood == Mood::Night {
                for star in 0..23 {
                    let x = offset + 9 + (star * 37) % 368;
                    let y = top + 5 + (star * 19) % 58;
                    let color = if (index / TICK_FRAMES + star as usize * 7) % 48 < 24 {
                        [226, 230, 209]
                    } else {
                        [112, 143, 166]
                    };
                    frame.pixel(x, y, -5500, color);
                }
            }
            for &row in &down {
                for glint in 0..32 {
                    let x = offset + 4 + (glint * 37) % 376 + wave(index, glint as usize * 7, 2);
                    let y = row + 124 + (glint * 13) % 104;
                    let color = if (index / TICK_FRAMES + glint as usize * 3) % 48 < 24 {
                        [126, 180, 185]
                    } else {
                        [80, 145, 159]
                    };
                    frame.rect(x, y, 3, 1, -7000, atmosphere(color, self.mood));
                }
            }
            for bird in 0..5 {
                let x = offset + 26 + bird * 75 + wave(index, bird as usize * 9, 8);
                let y = top + 54 + wave(index, bird as usize * 11, 2);
                frame.line((x - 2, y), (x, y + 1), 4000, self.setting.materials().flier);
                frame.line(
                    (x, y + 1),
                    (x + 2, y + wave(index, 0, 1)),
                    4000,
                    self.setting.materials().flier,
                );
            }
        }
        for boat in (0..2).filter(|_| scene && self.setting.materials().boats) {
            let anchor = if boat == 0 { 22 } else { canvas_width - 25 };
            let x = left + anchor + wave(index, boat as usize * 32, 3);
            let y = top + 140 + boat * 8 + wave(index, boat as usize * 24, 1);
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
        self.ambient(frame, index);
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
/// Fixed parts of each setting's backdrop, drawn behind the island.
fn backdrop_details(frame: &mut Frame, setting: Setting, mood: Mood, seed: u64) {
    let (left, top) = (frame.left, frame.top);
    let (width, height) = (frame.width as i32, frame.height as i32);
    match setting {
        Setting::Underwater => {
            // Slanted shafts of light from the surface.
            for shaft in (0..width / 70 + 1).map(|i| left + i * 70 + (seed % 23) as i32) {
                for row in 0..height.min(135) {
                    frame.rect(
                        shaft + row / 3,
                        top + row,
                        9 - row / 15,
                        1,
                        -9000,
                        atmosphere([58, 138, 176], mood),
                    );
                }
            }
        }
        Setting::Underground => {
            // Stalactites hang from a dark ceiling.
            for tip in 0..width / 16 + 1 {
                let x = left + tip * 16 + 3;
                let reach = 10 + (stable_hash(&format!("{seed}:{tip}")) % 28) as i32;
                for row in 0..reach {
                    let half = (reach - row) / 5;
                    frame.rect(
                        x - half,
                        top + row,
                        half * 2 + 1,
                        1,
                        -8000,
                        atmosphere([62, 58, 60], mood),
                    );
                }
            }
        }
        Setting::Futuristic => {
            // A skyline of lit towers along the horizon.
            for tower in 0..width / 14 + 1 {
                let rise = 12 + (stable_hash(&format!("{seed}:{tower}")) % 34) as i32;
                let x = left + tower * 14;
                frame.rect(
                    x,
                    top + 92 - rise,
                    11,
                    rise,
                    -8000,
                    atmosphere([30, 22, 64], mood),
                );
                frame.rect(
                    x + 2,
                    top + 92 - rise + 3,
                    2,
                    1,
                    -7900,
                    atmosphere([240, 110, 210], mood),
                );
            }
        }
        _ => {}
    }
}
/// A canvas holding the whole city with room for roofs and fireworks above it, never smaller than the base canvas.
fn city_canvas(plan: &CityPlan, scene: bool, fill: [u8; 3]) -> Frame {
    let (mut low_x, mut low_y, mut high_x, mut high_y) = (0, 0, WIDTH as i32, HEIGHT as i32);
    let strip = if scene { 22 } else { 0 };
    for &(x, y) in &plan.land {
        let (sx, sy) = project(x, y, 0);
        low_x = low_x.min(sx - 8);
        high_x = high_x.max(sx + 8);
        low_y = low_y.min(sy - 8);
        high_y = high_y.max(sy + 18 + strip);
    }
    for building in &plan.buildings {
        let (_, sy) = project(building.x, building.y, building.height + 18);
        low_y = low_y.min(sy - 34);
    }
    Frame::canvas(
        low_x,
        low_y,
        (high_x - low_x) as usize,
        (high_y - low_y) as usize,
        fill,
    )
}
fn pine(frame: &mut Frame, x: i32, y: i32, setting: Setting, mood: Mood) {
    let (sx, sy) = project(x, y, 0);
    let depth = (x + y) * 8 + 8;
    let foliage = setting.materials().foliage;
    frame.rect(
        sx,
        sy - 2,
        1,
        3,
        depth,
        atmosphere(setting.materials().trunk, mood),
    );
    for row in 0..8 {
        let half = (row + 1) / 2;
        let tone = if row % 3 == 2 { 88 } else { 74 };
        frame.rect(
            sx - half,
            sy - 10 + row,
            half * 2 + 1,
            1,
            depth,
            atmosphere(shade(foliage, tone), mood),
        );
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
    let foliage = setting.materials().foliage;
    match setting.materials().flora {
        Flora::Trees => {}
        Flora::Rocks => {
            disc(frame, sx, sy - 2, 3, depth, atmosphere(foliage, mood));
            disc(
                frame,
                sx - 1,
                sy - 3,
                1,
                depth + 1,
                atmosphere(shade(foliage, 130), mood),
            );
            return;
        }
        Flora::Pylons => {
            frame.rect(
                sx,
                sy - 10,
                1,
                11,
                depth,
                atmosphere(setting.materials().trunk, mood),
            );
            frame.rect(sx - 1, sy - 12, 3, 2, depth + 1, atmosphere(foliage, mood));
            return;
        }
        Flora::Kelp => {
            for row in 0..11 {
                let bend = if (row + x) % 4 < 2 { 0 } else { 1 };
                frame.rect(sx + bend, sy - row, 2, 1, depth, atmosphere(foliage, mood));
            }
            return;
        }
        Flora::Crystals => {
            for (dx, height, tone) in [(-2, 4, 80), (0, 8, 100), (2, 5, 125)] {
                frame.rect(
                    sx + dx,
                    sy - height,
                    2,
                    height + 1,
                    depth,
                    atmosphere(shade(foliage, tone), mood),
                );
            }
            return;
        }
    }
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
fn draw_vehicle(
    frame: &mut Frame,
    sections: &[crate::motion::Pose],
    mode: Mode,
    color: [u8; 3],
    lift: i32,
) {
    let height = if mode == Mode::Bicycle { 2 } else { 3 };
    let length = sections.len();
    for (section, body) in sections.iter().enumerate() {
        let (vx, vy, z) = (body.x, body.y, body.z + lift);
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
    let materials = setting.materials();
    let [upper, lower] = materials.soil;
    // Every exposed front edge gets its own depth-tested earth face, including concave coasts.
    for &(x, y) in &plan.land {
        // Solid tile columns also cover interior samples that can be visible through a concave projected coast.
        for below in 0..=9 {
            let (sx, sy) = project(x, y, -below);
            let soil = atmosphere(if below < 5 { upper } else { lower }, mood);
            frame.pixel(sx, sy, (x + y) * 8 - below, soil);
            frame.pixel(sx, sy + 1, (x + y) * 8 - below, soil);
        }
    }
    for &(x, y) in &plan.land {
        for (dx, dy, tone) in [(1, 0, 100), (0, 1, 76)] {
            if !plan.land.contains(&(x + dx, y + dy)) {
                // Rasterize the entire tile edge, including its shared corner. A single sample per tile leaves alternating empty screen columns.
                for below in 0..=9 {
                    let soil = if below < 5 { upper } else { lower };
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
            materials.rim
        } else if setting == Setting::Futuristic
            && plan.surface(x, y) == Surface::Grass
            && (x.rem_euclid(8) == 0 || y.rem_euclid(8) == 0)
        {
            [46, 92, 128]
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
