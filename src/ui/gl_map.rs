use std::collections::HashSet;

use glam::{Mat4, Vec3};
use glow::HasContext;

use crate::core::easing::linear;
use crate::core::solar_system_manager::SolarSystemManager;
use crate::rendering::mouse_ray;
use crate::rendering::shader::Shader;
use crate::rendering::texture_loader;
use crate::resources;

pub enum MapLabelText {
    SystemName(usize),
    RegionName(usize),
    CharName(usize),
    Owned(String),
}

pub struct MapLabel {
    pub x: f32,
    pub y: f32,
    pub text: MapLabelText,
    pub color: [f32; 4],
}

fn project_to_screen(mvp: &Mat4, world: [f32; 3], vw: f32, vh: f32) -> Option<(f32, f32)> {
    let clip = *mvp * glam::Vec4::new(world[0], world[1], world[2], 1.0);
    if clip.w <= 0.0 {
        return None;
    }
    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;
    Some(((ndc_x * 0.5 + 0.5) * vw, (1.0 - (ndc_y * 0.5 + 0.5)) * vh))
}

pub struct GlMap {
    pub camera_distance: f32,
    pub look_at: [f32; 3],

    projection: Mat4,
    modelview: Mat4,

    zooming: bool,
    zoom_tick: f32,
    max_zoom_tick: f32,
    zoom_start: [f32; 3],
    zoom_end: [f32; 3],

    gl_loaded: bool,
    shader_systems: Option<Shader>,
    shader_conn: Option<Shader>,
    shader_crosshair: Option<Shader>,

    system_vao: Option<glow::VertexArray>,
    system_vbo: Option<glow::Buffer>,
    color_vbo: Option<glow::Buffer>,
    conn_vao: Option<glow::VertexArray>,
    conn_vbo: Option<glow::Buffer>,
    conn_color_vbo: Option<glow::Buffer>,
    crosshair_vao: Option<glow::VertexArray>,
    crosshair_vbo: Option<glow::Buffer>,

    tex_system: Option<glow::Texture>,
    tex_green_ch: Option<glow::Texture>,
    tex_red_ch: Option<glow::Texture>,
    tex_yellow_ch: Option<glow::Texture>,
    tex_red_green_ch: Option<glow::Texture>,
    tex_red_yellow_ch: Option<glow::Texture>,
    tex_yellow_green_ch: Option<glow::Texture>,

    pub pending_labels: Vec<MapLabel>,

    pub point_size: f32,
    pub crosshair_size: f32,
    pub map_text_size: u32,
    pub persistent_labels: bool,
    pub show_alert_age: bool,
    pub display_char_names: bool,
    pub show_char_locations: bool,
    pub sticky_highlights: HashSet<usize>,
    pub landmark_systems: HashSet<usize>,
    pub char_locations: Vec<(String, usize)>,
    pub scroll_sensitivity: f32,
    pub map_mode_2d: bool,

    pub hovered_system: Option<usize>,
    pub hovered_connections: Vec<usize>,

    cached_flat_colors: Vec<f32>,
    vbos_uploaded: bool,
}

impl GlMap {
    pub fn new() -> Self {
        Self {
            camera_distance: 2000.0,
            look_at: [-1416.0, 3702.0, 0.0],
            projection: Mat4::IDENTITY,
            modelview: Mat4::IDENTITY,
            zooming: false,
            zoom_tick: 0.0,
            max_zoom_tick: 100.0,
            zoom_start: [0.0; 3],
            zoom_end: [0.0; 3],
            gl_loaded: false,
            shader_systems: None,
            shader_conn: None,
            shader_crosshair: None,
            system_vao: None,
            system_vbo: None,
            color_vbo: None,
            conn_vao: None,
            conn_vbo: None,
            conn_color_vbo: None,
            crosshair_vao: None,
            crosshair_vbo: None,
            tex_system: None,
            tex_green_ch: None,
            tex_red_ch: None,
            tex_yellow_ch: None,
            tex_red_green_ch: None,
            tex_red_yellow_ch: None,
            tex_yellow_green_ch: None,
            pending_labels: Vec::new(),
            point_size: 1.0,
            crosshair_size: 26.0,
            scroll_sensitivity: 1.0,
            map_text_size: 20,
            persistent_labels: false,
            show_alert_age: true,
            display_char_names: true,
            show_char_locations: true,
            sticky_highlights: HashSet::new(),
            landmark_systems: HashSet::new(),
            char_locations: Vec::new(),
            map_mode_2d: false,
            hovered_system: None,
            hovered_connections: Vec::new(),
            cached_flat_colors: Vec::new(),
            vbos_uploaded: false,
        }
    }

    pub fn init_gl(&mut self, gl: &glow::Context) {
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.enable(glow::DEPTH_TEST);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.enable(glow::PROGRAM_POINT_SIZE);
        }

        self.load_shaders(gl);
        self.load_textures(gl);
        self.gl_loaded = true;
    }

    fn load_shaders(&mut self, gl: &glow::Context) {
        match Shader::new(gl, resources::SHADER_VERT, resources::SHADER_FRAG) {
            Ok(s) => self.shader_systems = Some(s),
            Err(e) => log::error!("System shader failed: {}", e),
        }
        match Shader::new(gl, resources::CONNECTION_VERT, resources::CONNECTION_FRAG) {
            Ok(s) => self.shader_conn = Some(s),
            Err(e) => log::error!("Connection shader failed: {}", e),
        }
        match Shader::new(gl, resources::CROSSHAIR_VERT, resources::CROSSHAIR_FRAG) {
            Ok(s) => self.shader_crosshair = Some(s),
            Err(e) => log::error!("Crosshair shader failed: {}", e),
        }
    }

    fn load_textures(&mut self, gl: &glow::Context) {
        self.tex_system = texture_loader::load_texture_from_bytes(gl, resources::TEX_SYSTEM);
        self.tex_green_ch = texture_loader::load_texture_from_bytes(gl, resources::TEX_GREEN_CH);
        self.tex_red_ch = texture_loader::load_texture_from_bytes(gl, resources::TEX_RED_CH);
        self.tex_yellow_ch = texture_loader::load_texture_from_bytes(gl, resources::TEX_YELLOW_CH);
        self.tex_red_green_ch = texture_loader::load_texture_from_bytes(gl, resources::TEX_RED_GREEN_CH);
        self.tex_red_yellow_ch = texture_loader::load_texture_from_bytes(gl, resources::TEX_RED_YELLOW_CH);
        self.tex_yellow_green_ch = texture_loader::load_texture_from_bytes(gl, resources::TEX_YELLOW_GREEN_CH);
    }

    pub fn init_vbos(&mut self, gl: &glow::Context, manager: &SolarSystemManager) {
        if manager.system_count() == 0 {
            return;
        }
        log::info!("init_vbos: {} systems, {} connection vertices, shader_conn={}",
            manager.system_count(), manager.connection_vertex_count, self.shader_conn.is_some());

        unsafe {
            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let pos_bytes: &[u8] = std::slice::from_raw_parts(
                manager.system_positions.as_ptr() as *const u8,
                manager.system_positions.len() * 4,
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, pos_bytes, glow::STATIC_DRAW);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(0);

            let color_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(color_vbo));
            let color_bytes: &[u8] = std::slice::from_raw_parts(
                manager.system_colors.as_ptr() as *const u8,
                manager.system_colors.len() * 4,
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, color_bytes, glow::STATIC_DRAW);
            gl.vertex_attrib_pointer_f32(1, 4, glow::UNSIGNED_BYTE, true, 0, 0);
            gl.enable_vertex_attrib_array(1);

            gl.bind_vertex_array(None);

            self.system_vao = Some(vao);
            self.system_vbo = Some(vbo);
            self.color_vbo = Some(color_vbo);

            if manager.connection_vertex_count > 0 {
                let conn_vao = gl.create_vertex_array().unwrap();
                gl.bind_vertex_array(Some(conn_vao));

                let conn_vbo = gl.create_buffer().unwrap();
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(conn_vbo));
                let conn_bytes: &[u8] = std::slice::from_raw_parts(
                    manager.connection_positions.as_ptr() as *const u8,
                    manager.connection_positions.len() * 4,
                );
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, conn_bytes, glow::STATIC_DRAW);
                gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0);
                gl.enable_vertex_attrib_array(0);

                let conn_color_vbo = gl.create_buffer().unwrap();
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(conn_color_vbo));
                let conn_color_bytes: &[u8] = std::slice::from_raw_parts(
                    manager.connection_colors.as_ptr() as *const u8,
                    manager.connection_colors.len() * 4,
                );
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, conn_color_bytes, glow::STATIC_DRAW);
                gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 0, 0);
                gl.enable_vertex_attrib_array(1);

                gl.bind_vertex_array(None);

                self.conn_vao = Some(conn_vao);
                self.conn_vbo = Some(conn_vbo);
                self.conn_color_vbo = Some(conn_color_vbo);
            }

            let ch_vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(ch_vao));
            let ch_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(ch_vbo));
            let dummy = [0.0f32; 3];
            let dummy_bytes: &[u8] =
                std::slice::from_raw_parts(dummy.as_ptr() as *const u8, 12);
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, dummy_bytes, glow::DYNAMIC_DRAW);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(0);
            gl.bind_vertex_array(None);

            self.crosshair_vao = Some(ch_vao);
            self.crosshair_vbo = Some(ch_vbo);
        }

        self.vbos_uploaded = true;
    }

    pub fn update_vbos(&mut self, gl: &glow::Context, manager: &mut SolarSystemManager) {
        manager.refresh_vbo_data();

        unsafe {
            if manager.is_system_vbo_dirty {
                if let Some(vbo) = self.system_vbo {
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    let bytes: &[u8] = std::slice::from_raw_parts(
                        manager.system_positions.as_ptr() as *const u8,
                        manager.system_positions.len() * 4,
                    );
                    gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytes);
                }
                manager.is_system_vbo_dirty = false;
            }

            if manager.is_color_vao_dirty {
                if let Some(vbo) = self.color_vbo {
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    let bytes: &[u8] = std::slice::from_raw_parts(
                        manager.system_colors.as_ptr() as *const u8,
                        manager.system_colors.len() * 4,
                    );
                    gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytes);
                }
                manager.is_color_vao_dirty = false;
            }

            if manager.is_connection_vbo_dirty && manager.connection_vertex_count > 0 {
                if let (Some(vbo), Some(cvbo)) = (self.conn_vbo, self.conn_color_vbo) {
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    let pos_bytes: &[u8] = std::slice::from_raw_parts(
                        manager.connection_positions.as_ptr() as *const u8,
                        manager.connection_positions.len() * 4,
                    );
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, pos_bytes, glow::STATIC_DRAW);

                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(cvbo));
                    let color_bytes: &[u8] = std::slice::from_raw_parts(
                        manager.connection_colors.as_ptr() as *const u8,
                        manager.connection_colors.len() * 4,
                    );
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, color_bytes, glow::STATIC_DRAW);
                } else if self.conn_vao.is_none() {
                    let conn_vao = gl.create_vertex_array().unwrap();
                    gl.bind_vertex_array(Some(conn_vao));

                    let conn_vbo = gl.create_buffer().unwrap();
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(conn_vbo));
                    let pos_bytes: &[u8] = std::slice::from_raw_parts(
                        manager.connection_positions.as_ptr() as *const u8,
                        manager.connection_positions.len() * 4,
                    );
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, pos_bytes, glow::STATIC_DRAW);
                    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0);
                    gl.enable_vertex_attrib_array(0);

                    let conn_color_vbo = gl.create_buffer().unwrap();
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(conn_color_vbo));
                    let color_bytes: &[u8] = std::slice::from_raw_parts(
                        manager.connection_colors.as_ptr() as *const u8,
                        manager.connection_colors.len() * 4,
                    );
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, color_bytes, glow::STATIC_DRAW);
                    gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 0, 0);
                    gl.enable_vertex_attrib_array(1);

                    gl.bind_vertex_array(None);

                    self.conn_vao = Some(conn_vao);
                    self.conn_vbo = Some(conn_vbo);
                    self.conn_color_vbo = Some(conn_color_vbo);
                }
                manager.is_connection_vbo_dirty = false;
            }
        }
    }

    pub fn paint(&mut self, gl: &glow::Context, manager: &mut SolarSystemManager, w: f32, h: f32) {
        if !self.gl_loaded || !self.vbos_uploaded {
            return;
        }

        unsafe {
            gl.clear(glow::DEPTH_BUFFER_BIT);
        }

        let aspect = w / h.max(1.0);
        self.projection = Mat4::perspective_rh_gl(45.0_f32.to_radians(), aspect, 1.0, 50000.0);

        let eye = Vec3::new(self.look_at[0], self.look_at[1], self.camera_distance);
        let center = Vec3::from(self.look_at);
        let up = Vec3::Y;
        self.modelview = Mat4::look_at_rh(eye, center, up);

        if self.zooming {
            self.zoom_tick += 1.0;
            if self.zoom_tick >= self.max_zoom_tick {
                self.zooming = false;
                self.look_at = self.zoom_end;
            } else {
                for i in 0..3 {
                    self.look_at[i] = linear(
                        self.zoom_tick,
                        self.zoom_start[i],
                        self.zoom_end[i] - self.zoom_start[i],
                        self.max_zoom_tick,
                    );
                }
            }
        }

        self.update_vbos(gl, manager);

        if !manager.are_uniforms_clean {
            manager.build_uniforms();
        }

        self.point_size = (6000.0 / self.camera_distance).max(1.0);
        self.crosshair_size = (16000.0 / self.camera_distance).clamp(26.0, 52.0);

        if let Some(ref mut shader) = self.shader_conn {
            if manager.connection_vertex_count > 0 {
                if let Some(vao) = self.conn_vao {
                    shader.bind(gl);
                    shader.set_mat4(gl, "projection", &self.projection.to_cols_array());
                    shader.set_mat4(gl, "modelView", &self.modelview.to_cols_array());
                    unsafe {
                        gl.line_width(1.5);
                        gl.bind_vertex_array(Some(vao));
                        gl.draw_arrays(glow::LINES, 0, manager.connection_vertex_count as i32);
                        gl.bind_vertex_array(None);
                    }
                    Shader::unbind(gl);
                }
            }
        }

        if let Some(ref mut shader) = self.shader_systems {
            if let Some(vao) = self.system_vao {
                shader.bind(gl);
                shader.set_mat4(gl, "projection", &self.projection.to_cols_array());
                shader.set_mat4(gl, "modelView", &self.modelview.to_cols_array());
                shader.set_1f(gl, "pointsize", self.point_size);

                shader.set_1iv(gl, "hlpoints", &manager.uni_system_ids);
                shader.set_1fv(gl, "hlsizes", &manager.uni_sizes);

                if self.cached_flat_colors.is_empty() {
                    self.cached_flat_colors.clear();
                    for c in &manager.uni_colors {
                        self.cached_flat_colors.extend_from_slice(c);
                    }
                }
                shader.set_1fv(gl, "hlcolors", &self.cached_flat_colors);

                if let Some(tex) = self.tex_system {
                    shader.bind_texture(gl, tex, 0, "tex");
                }

                unsafe {
                    gl.bind_vertex_array(Some(vao));
                    gl.draw_arrays(glow::POINTS, 0, manager.system_count() as i32);
                    gl.bind_vertex_array(None);
                }
                Shader::unbind(gl);
            }
        }

        self.draw_crosshairs(gl, manager);
        self.build_labels(manager, w, h);
    }

    fn draw_crosshairs(&mut self, gl: &glow::Context, manager: &SolarSystemManager) {
        let Some(ref mut shader) = self.shader_crosshair else {
            return;
        };
        let Some(ch_vao) = self.crosshair_vao else {
            return;
        };
        let Some(ch_vbo) = self.crosshair_vbo else {
            return;
        };

        let proj = self.projection.to_cols_array();
        let mv = self.modelview.to_cols_array();
        let ch_size = self.crosshair_size;

        unsafe {
            gl.disable(glow::DEPTH_TEST);
        }

        let draw_one = |shader: &mut Shader,
                        gl: &glow::Context,
                        sys_id: usize,
                        tex: Option<glow::Texture>,
                        systems: &[crate::core::solar_system::SolarSystem]| {
            let Some(texture) = tex else { return };
            if sys_id >= systems.len() {
                return;
            }
            let pos = systems[sys_id].xyz();
            unsafe {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(ch_vbo));
                let bytes: &[u8] =
                    std::slice::from_raw_parts(pos.as_ptr() as *const u8, 12);
                gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytes);
            }

            shader.bind(gl);
            shader.set_mat4(gl, "projection", &proj);
            shader.set_mat4(gl, "modelView", &mv);
            shader.set_1f(gl, "pointsize", ch_size);
            shader.bind_texture(gl, texture, 0, "tex");

            unsafe {
                gl.bind_vertex_array(Some(ch_vao));
                gl.draw_arrays(glow::POINTS, 0, 1);
                gl.bind_vertex_array(None);
            }
            Shader::unbind(gl);
        };

        for &sys_id in &manager.green_crosshair_ids {
            draw_one(shader, gl, sys_id, self.tex_green_ch, &manager.solar_systems);
        }

        for &sys_id in &manager.red_crosshair_ids {
            let is_green = manager.green_crosshair_ids.contains(&sys_id);
            let tex = if is_green {
                self.tex_red_green_ch
            } else {
                self.tex_red_ch
            };
            draw_one(shader, gl, sys_id, tex, &manager.solar_systems);
        }

        if self.show_char_locations {
            for &(_, sys_id) in &self.char_locations {
                let is_red = manager.red_crosshair_ids.contains(&sys_id);
                let is_green = manager.green_crosshair_ids.contains(&sys_id);
                let tex = if is_red {
                    self.tex_red_yellow_ch
                } else if is_green {
                    self.tex_yellow_green_ch
                } else {
                    self.tex_yellow_ch
                };
                draw_one(shader, gl, sys_id, tex, &manager.solar_systems);
            }
        }

        unsafe {
            gl.enable(glow::DEPTH_TEST);
        }
    }

    pub fn zoom_to_system(&mut self, system_id: usize, systems: &[crate::core::solar_system::SolarSystem]) {
        if system_id >= systems.len() {
            return;
        }
        self.zoom_start = self.look_at;
        self.zoom_end = systems[system_id].xyz();
        self.zoom_tick = 0.0;
        self.zooming = true;
    }

    pub fn pan_to_system(&mut self, system_id: usize, systems: &[crate::core::solar_system::SolarSystem]) {
        if system_id >= systems.len() {
            return;
        }
        let pos = systems[system_id].xyz();
        self.look_at = pos;
    }

    pub fn handle_scroll(&mut self, delta: f32) {
        let normalized = delta / 50.0;
        let speed = 0.05 * self.scroll_sensitivity;
        self.camera_distance = (self.camera_distance * (1.0 - normalized * speed))
            .clamp(10.0, 15000.0);
    }

    pub fn handle_drag(&mut self, dx: f32, dy: f32) {
        let scale = self.camera_distance / 1000.0;
        self.look_at[0] -= dx * scale;
        self.look_at[1] += dy * scale;
    }

    pub fn pick_system(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        viewport_w: f32,
        viewport_h: f32,
        systems: &[crate::core::solar_system::SolarSystem],
    ) -> Option<usize> {
        let (origin, dir) = mouse_ray::unproject(
            mouse_x,
            mouse_y,
            viewport_w,
            viewport_h,
            &self.projection,
            &self.modelview,
        );

        let pick_radius = (self.camera_distance / 200.0).max(5.0);
        let mut best: Option<(usize, f32)> = None;

        for (i, sys) in systems.iter().enumerate() {
            let center = Vec3::new(sys.x, sys.y, sys.z);
            if mouse_ray::ray_sphere_intersect(origin, dir, center, pick_radius) {
                let dist = (center - origin).length_squared();
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((i, dist));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    pub fn invalidate_uniforms(&mut self) {
        self.cached_flat_colors.clear();
    }

    fn build_labels(&mut self, manager: &SolarSystemManager, w: f32, h: f32) {
        self.pending_labels.clear();

        let mvp = self.projection * self.modelview;
        let label_offset_x = self.crosshair_size * 0.6;
        let label_offset_y = -(self.map_text_size as f32) * 0.6;
        let line_h = self.map_text_size as f32 * 1.2;

        for &sys_id in &manager.red_crosshair_ids {
            if sys_id >= manager.solar_systems.len() { continue; }
            let sys = &manager.solar_systems[sys_id];
            let Some((sx, sy)) = project_to_screen(&mvp, sys.xyz(), w, h) else { continue };

            self.pending_labels.push(MapLabel {
                x: sx + label_offset_x, y: sy + label_offset_y,
                text: MapLabelText::SystemName(sys_id), color: [1.0, 0.3, 0.3, 1.0],
            });

            if self.show_alert_age {
                if let Some(stats) = manager.system_stats.get(&sys_id) {
                    let elapsed = chrono::Utc::now() - stats.last_report;
                    let secs = elapsed.num_seconds().max(0);
                    self.pending_labels.push(MapLabel {
                        x: sx + label_offset_x, y: sy + label_offset_y + line_h,
                        text: MapLabelText::Owned(format!("{}:{:02}", secs / 60, secs % 60)),
                        color: [1.0, 0.5, 0.5, 1.0],
                    });
                }
            }
        }

        for &sys_id in &manager.green_crosshair_ids {
            if sys_id >= manager.solar_systems.len() { continue; }
            if manager.red_crosshair_ids.contains(&sys_id) { continue; }
            let sys = &manager.solar_systems[sys_id];
            let Some((sx, sy)) = project_to_screen(&mvp, sys.xyz(), w, h) else { continue };
            self.pending_labels.push(MapLabel {
                x: sx + label_offset_x, y: sy + label_offset_y,
                text: MapLabelText::SystemName(sys_id), color: [0.0, 0.8, 0.0, 1.0],
            });
        }

        if self.show_char_locations && self.display_char_names {
            let mut char_count_at: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
            for (i, (_, sys_id)) in self.char_locations.iter().enumerate() {
                if *sys_id >= manager.solar_systems.len() { continue; }
                let sys = &manager.solar_systems[*sys_id];
                let Some((sx, sy)) = project_to_screen(&mvp, sys.xyz(), w, h) else { continue };
                let slot = char_count_at.entry(*sys_id).or_insert(0);
                self.pending_labels.push(MapLabel {
                    x: sx + label_offset_x, y: sy + label_offset_y - line_h - (*slot as f32 * line_h),
                    text: MapLabelText::CharName(i), color: [0.0, 0.6, 0.8, 1.0],
                });
                *slot += 1;
            }
        }

        if self.persistent_labels {
            for &sys_id in &self.landmark_systems {
                if sys_id >= manager.solar_systems.len() { continue; }
                if manager.red_crosshair_ids.contains(&sys_id) { continue; }
                if manager.green_crosshair_ids.contains(&sys_id) { continue; }
                let sys = &manager.solar_systems[sys_id];
                let Some((sx, sy)) = project_to_screen(&mvp, sys.xyz(), w, h) else { continue };
                self.pending_labels.push(MapLabel {
                    x: sx, y: sy - line_h,
                    text: MapLabelText::SystemName(sys_id), color: [0.7, 0.7, 0.7, 0.8],
                });
            }
        }

        for &sys_id in &self.sticky_highlights {
            if sys_id >= manager.solar_systems.len() { continue; }
            if manager.red_crosshair_ids.contains(&sys_id) { continue; }
            if manager.green_crosshair_ids.contains(&sys_id) { continue; }
            let sys = &manager.solar_systems[sys_id];
            let Some((sx, sy)) = project_to_screen(&mvp, sys.xyz(), w, h) else { continue };
            self.pending_labels.push(MapLabel {
                x: sx + label_offset_x, y: sy + label_offset_y,
                text: MapLabelText::SystemName(sys_id), color: [1.0, 1.0, 1.0, 1.0],
            });
        }

        if self.camera_distance > 500.0 {
            let alpha = ((self.camera_distance - 500.0) / 500.0).min(1.0);
            for (i, region) in manager.region_labels.iter().enumerate() {
                let Some((sx, sy)) = project_to_screen(&mvp, [region.x, region.y, region.z], w, h) else { continue };
                self.pending_labels.push(MapLabel {
                    x: sx, y: sy,
                    text: MapLabelText::RegionName(i), color: [0.5, 0.5, 0.6, alpha * 0.6],
                });
            }
        }

        for &sys_id in &self.hovered_connections {
            if sys_id >= manager.solar_systems.len() { continue; }
            if manager.red_crosshair_ids.contains(&sys_id) { continue; }
            if manager.green_crosshair_ids.contains(&sys_id) { continue; }
            let sys = &manager.solar_systems[sys_id];
            let Some((sx, sy)) = project_to_screen(&mvp, sys.xyz(), w, h) else { continue };
            self.pending_labels.push(MapLabel {
                x: sx, y: sy - line_h,
                text: MapLabelText::SystemName(sys_id), color: [0.5, 0.5, 0.5, 0.6],
            });
        }

        if let Some(sys_id) = self.hovered_system {
            if sys_id < manager.solar_systems.len() {
                let sys = &manager.solar_systems[sys_id];
                if let Some((sx, sy)) = project_to_screen(&mvp, sys.xyz(), w, h) {
                    self.pending_labels.push(MapLabel {
                        x: sx + label_offset_x, y: sy + label_offset_y,
                        text: MapLabelText::SystemName(sys_id), color: [1.0, 1.0, 0.0, 1.0],
                    });
                }
            }
        }
    }
}
