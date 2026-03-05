use crate::core::easing::{quint_ease_in, quint_ease_out};

pub const DEFAULT_DRAW_COLOR: [u8; 4] = [172, 207, 243, 255];
pub const HIGHLIGHT_DRAW_COLOR: [u8; 4] = [255, 255, 255, 255];
pub const ALERTING_DRAW_COLOR: [u8; 4] = [255, 0, 0, 255];
pub const CHARACTER_LOCATION_DRAW_COLOR: [u8; 4] = [0, 200, 0, 255];
pub const CHARACTER_ALERT_DRAW_COLOR: [u8; 4] = [255, 140, 0, 255];

pub fn color_to_rgba32(c: [u8; 4]) -> u32 {
    let [r, g, b, a] = c;
    (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | (r as u32)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationState {
    Growing,
    Paused,
    Shrinking,
    Idle,
}

#[derive(Clone, Copy, Debug)]
pub struct SolarSystemConnection {
    pub to_system_id: usize,
    pub is_regional: bool,
}

pub struct SolarSystem {
    pub native_id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub x3d: f32,
    pub y3d: f32,
    pub z3d: f32,
    pub x2d: f32,
    pub y2d: f32,
    pub region_id: u32,
    pub connected_to: Vec<SolarSystemConnection>,

    pub is_highlighted: bool,
    pub is_alerting: bool,

    pub alert_state: AnimationState,
    pub highlight_state: AnimationState,
    pub draw_size: f32,
    pub draw_color: [u8; 4],

    step_alert: i32,
    step_highlight: i32,
    alert_pulse_count: i32,
    is_flash: bool,

    name_lower: String,
}

impl SolarSystem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        native_id: u32,
        name: String,
        x: f32,
        y: f32,
        z: f32,
        x2d: f32,
        y2d: f32,
        region_id: u32,
    ) -> Self {
        let name_lower = name.to_lowercase();

        Self {
            native_id,
            name,
            x,
            y,
            z,
            x3d: x,
            y3d: y,
            z3d: z,
            x2d,
            y2d,
            region_id,
            connected_to: Vec::new(),
            is_highlighted: false,
            is_alerting: false,
            alert_state: AnimationState::Idle,
            highlight_state: AnimationState::Idle,
            draw_size: 0.0,
            draw_color: DEFAULT_DRAW_COLOR,
            step_alert: 1,
            step_highlight: 0,
            alert_pulse_count: 0,
            is_flash: false,
            name_lower,
        }
    }

    pub fn xyz(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    pub fn set_map_mode(&mut self, mode_2d: bool) {
        if mode_2d {
            self.x = self.x2d;
            self.y = self.y2d;
            self.z = 0.0;
        } else {
            self.x = self.x3d;
            self.y = self.y3d;
            self.z = self.z3d;
        }
    }

    // Whole-word match against pre-lowered text
    pub fn match_name_in_lower(&self, haystack_lower: &str) -> bool {
        let needle = &self.name_lower;
        if needle.is_empty() {
            return false;
        }
        let hay = haystack_lower.as_bytes();
        let nlen = needle.len();
        let mut start = 0;
        while let Some(pos) = haystack_lower[start..].find(needle.as_str()) {
            let abs_pos = start + pos;
            let end_pos = abs_pos + nlen;
            let before_ok = abs_pos == 0 || !hay[abs_pos - 1].is_ascii_alphanumeric();
            let after_ok = end_pos >= hay.len() || !hay[end_pos].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
            start = abs_pos + 1;
        }
        false
    }

    pub fn draw_color_rgba_floats(&self) -> [f32; 4] {
        [
            self.draw_color[0] as f32 / 255.0,
            self.draw_color[1] as f32 / 255.0,
            self.draw_color[2] as f32 / 255.0,
            self.draw_color[3] as f32 / 255.0,
        ]
    }

    pub fn draw_color_argb32(&self) -> u32 {
        color_to_rgba32(self.draw_color)
    }

    pub fn process_tick(&mut self) -> (bool, bool) {
        let htr = self.process_highlight_tick();
        let atr = self.process_alert_tick();
        (htr, atr)
    }

    fn process_alert_tick(&mut self) -> bool {
        let max_tick = 30;
        if self.step_alert > 0
            && self.step_alert < max_tick
            && self.alert_state != AnimationState::Idle
        {
            match self.alert_state {
                AnimationState::Growing => self.step_alert += 1,
                AnimationState::Shrinking => self.step_alert -= 1,
                _ => {}
            }
        }

        if self.step_alert >= max_tick && self.alert_state == AnimationState::Growing {
            self.alert_state = AnimationState::Shrinking;
            self.step_alert -= 1;
        } else if self.step_alert <= 0 && self.alert_state == AnimationState::Shrinking {
            self.alert_state = AnimationState::Growing;
            self.step_alert += 1;
            self.alert_pulse_count += 1;
        }

        if self.alert_pulse_count > 4 {
            self.alert_state = AnimationState::Idle;
            self.is_alerting = false;
            self.draw_color = DEFAULT_DRAW_COLOR;
            self.draw_size = 0.0;
            self.alert_pulse_count = 0;
            true
        } else {
            if matches!(
                self.alert_state,
                AnimationState::Growing | AnimationState::Shrinking
            ) {
                self.draw_size =
                    quint_ease_in(self.step_alert as f32, 1.0, 100.0, max_tick as f32);
            }
            false
        }
    }

    fn process_highlight_tick(&mut self) -> bool {
        let max_tick = 20;
        if self.step_highlight > 0
            && self.step_highlight < max_tick
            && self.highlight_state != AnimationState::Idle
        {
            match self.highlight_state {
                AnimationState::Growing => self.step_highlight += 1,
                AnimationState::Shrinking => self.step_highlight -= 1,
                _ => {}
            }
        }

        if self.step_highlight >= max_tick && self.highlight_state == AnimationState::Growing {
            self.highlight_state = if !self.is_flash {
                AnimationState::Paused
            } else {
                AnimationState::Shrinking
            };
            self.step_highlight -= 1;
        } else if self.step_highlight <= 0 && self.highlight_state == AnimationState::Shrinking {
            self.step_highlight += 1;
            self.highlight_state = AnimationState::Idle;
            self.is_highlighted = false;
            self.is_flash = false;
            self.draw_color = DEFAULT_DRAW_COLOR;
            self.draw_size = 0.0;
            return true;
        }

        if self.highlight_state == AnimationState::Growing {
            self.draw_size =
                quint_ease_out(self.step_highlight as f32, 1.0, 10.0, max_tick as f32);
        } else if self.highlight_state == AnimationState::Shrinking {
            self.draw_size =
                quint_ease_in(self.step_highlight as f32, 1.0, 10.0, max_tick as f32);
        }

        false
    }

    pub fn reset_highlight(&mut self) {
        self.step_highlight += 1;
        self.highlight_state = AnimationState::Idle;
        self.is_highlighted = false;
        self.draw_color = DEFAULT_DRAW_COLOR;
        self.draw_size = 0.0;
    }

    pub fn clear_alert(&mut self) {
        self.draw_size = 0.0;
    }

    pub fn clear_highlight(&mut self) {
        self.draw_size = 0.0;
    }

    pub fn start_alert(&mut self) {
        self.is_alerting = true;
        self.alert_state = AnimationState::Growing;
        self.draw_color = ALERTING_DRAW_COLOR;
        self.step_alert = 1;
    }

    pub fn start_highlight(&mut self, flash: bool) {
        self.is_highlighted = true;
        self.highlight_state = AnimationState::Growing;
        self.draw_color = HIGHLIGHT_DRAW_COLOR;
        self.is_flash = flash;
        self.step_highlight = 1;
    }
}
