use std::collections::{HashMap, VecDeque};

use chrono::{Duration, Utc};
use rustc_hash::FxHashMap;
use serde::Deserialize;

use crate::core::path_info::{generate_unique_path_id, PathInfo};
use crate::core::pathfinder::SolarSystemPathFinder;
use crate::core::solar_system::*;
use crate::core::system_stats::SystemStats;

#[derive(Deserialize)]
pub struct SystemDataJson {
    pub id: usize,
    pub native_id: u32,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(default)]
    pub x2d: Option<f64>,
    #[serde(default)]
    pub y2d: Option<f64>,
    #[serde(default)]
    pub region_id: u32,
    #[serde(default)]
    pub connected_to: Option<Vec<ConnectionDataJson>>,
}

#[derive(Deserialize)]
pub struct ConnectionDataJson {
    pub to_system_id: usize,
    #[allow(dead_code)]
    pub to_system_native_id: u32,
    #[serde(default)]
    pub is_regional: bool,
}

pub struct RegionLabel {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct SolarSystemManager {
    pub solar_systems: Vec<SolarSystem>,
    pub names: FxHashMap<String, usize>,

    pub system_positions: Vec<f32>,
    pub system_colors: Vec<u32>,
    pub system_elements: Vec<u32>,

    pub connection_positions: Vec<f32>,
    pub connection_colors: Vec<f32>,
    pub connection_vertex_count: usize,

    pub is_system_vbo_dirty: bool,
    pub is_color_vao_dirty: bool,
    pub is_connection_dirty: bool,
    pub is_connection_vbo_dirty: bool,

    pub home_system_id: i32,
    character_location: i32,
    pub character_location_systems: Vec<usize>,

    pub red_crosshair_ids: VecDeque<usize>,
    pub green_crosshair_ids: VecDeque<usize>,

    alert_systems: Vec<usize>,
    highlight_systems: Vec<usize>,

    pub uni_system_ids: [i32; 10],
    pub uni_colors: [[f32; 4]; 10],
    pub uni_sizes: [f32; 10],
    pub are_uniforms_clean: bool,

    pub path_finder: Option<SolarSystemPathFinder>,
    pathfinding_queue: VecDeque<PathInfo>,
    pub pathfinding_cache: FxHashMap<u64, PathInfo>,
    max_pathfinding_cache: usize,
    processing_path: bool,
    ok_to_process_paths: bool,

    pub system_stats: FxHashMap<usize, SystemStats>,
    pub max_alert_age: u32,
    pub max_alerts: usize,

    region_names: HashMap<u32, String>,
    pub region_labels: Vec<RegionLabel>,
    current_map_mode_2d: bool,
}

impl SolarSystemManager {
    pub fn new() -> Self {
        Self {
            solar_systems: Vec::new(),
            names: FxHashMap::default(),
            system_positions: Vec::new(),
            system_colors: Vec::new(),
            system_elements: Vec::new(),
            connection_positions: Vec::new(),
            connection_colors: Vec::new(),
            connection_vertex_count: 0,
            is_system_vbo_dirty: true,
            is_color_vao_dirty: true,
            is_connection_dirty: true,
            is_connection_vbo_dirty: false,
            home_system_id: -1,
            character_location: -1,
            character_location_systems: Vec::new(),
            red_crosshair_ids: VecDeque::with_capacity(50),
            green_crosshair_ids: VecDeque::with_capacity(10),
            alert_systems: Vec::new(),
            highlight_systems: Vec::new(),
            uni_system_ids: [-1; 10],
            uni_colors: [[1.0, 1.0, 1.0, 1.0]; 10],
            uni_sizes: [0.0; 10],
            are_uniforms_clean: false,
            path_finder: None,
            pathfinding_queue: VecDeque::new(),
            pathfinding_cache: FxHashMap::default(),
            max_pathfinding_cache: 5000,
            processing_path: false,
            ok_to_process_paths: false,
            system_stats: FxHashMap::default(),
            max_alert_age: 15,
            max_alerts: 15,
            region_names: HashMap::new(),
            region_labels: Vec::new(),
            current_map_mode_2d: false,
        }
    }

    pub fn system_count(&self) -> usize {
        self.solar_systems.len()
    }

    pub fn region_name(&self, region_id: u32) -> Option<&str> {
        self.region_names.get(&region_id).map(|s| s.as_str())
    }


    pub fn load_system_data(&mut self, data: Vec<SystemDataJson>) -> bool {
        let size = data.len();

        let mut all_connections: Vec<Vec<SolarSystemConnection>> = Vec::with_capacity(size);

        for sys_data in &data {
            let x = sys_data.x as f32;
            let y = sys_data.y as f32;
            let z = sys_data.z as f32;
            let x2d = sys_data.x2d.unwrap_or(sys_data.x) as f32;
            let y2d = sys_data.y2d.unwrap_or(sys_data.y) as f32;

            let mut solar = SolarSystem::new(
                sys_data.native_id,
                sys_data.name.clone(),
                x,
                y,
                z,
                x2d,
                y2d,
                sys_data.region_id,
            );

            let mut conns = Vec::new();
            if let Some(ref connected) = sys_data.connected_to {
                for conn_data in connected {
                    let conn = SolarSystemConnection {
                        to_system_id: conn_data.to_system_id,
                        is_regional: conn_data.is_regional,
                    };
                    solar.connected_to.push(conn);
                    conns.push(conn);
                }
            }

            self.names.insert(sys_data.name.clone(), sys_data.id);
            all_connections.push(conns);
            self.solar_systems.push(solar);
        }

        self.path_finder = Some(SolarSystemPathFinder::new(size, all_connections));
        self.ok_to_process_paths = true;

        self.system_count() > 0
    }

    pub fn load_region_names(&mut self, json_str: &str) {
        if let Ok(raw) = serde_json::from_str::<HashMap<String, String>>(json_str) {
            for (k, v) in raw {
                if let Ok(id) = k.parse::<u32>() {
                    self.region_names.insert(id, v);
                }
            }
        }
        self.compute_region_centroids();
    }

    fn compute_region_centroids(&mut self) {
        self.region_labels.clear();
        if self.region_names.is_empty() {
            return;
        }

        let mut region_sums: HashMap<u32, (f64, f64, f64, f64)> = HashMap::new();
        for system in &self.solar_systems {
            let rid = system.region_id;
            if rid == 0 || !self.region_names.contains_key(&rid) {
                continue;
            }
            let entry = region_sums.entry(rid).or_insert((0.0, 0.0, 0.0, 0.0));
            entry.0 += system.x as f64;
            entry.1 += system.y as f64;
            entry.2 += system.z as f64;
            entry.3 += 1.0;
        }

        for (rid, (sx, sy, sz, count)) in &region_sums {
            if *count > 0.0 {
                if let Some(name) = self.region_names.get(rid) {
                    self.region_labels.push(RegionLabel {
                        name: name.clone(),
                        x: (sx / count) as f32,
                        y: (sy / count) as f32,
                        z: (sz / count) as f32,
                    });
                }
            }
        }
    }

    pub fn set_map_mode(&mut self, mode_2d: bool) {
        self.current_map_mode_2d = mode_2d;
        for system in &mut self.solar_systems {
            system.set_map_mode(mode_2d);
        }
        self.is_system_vbo_dirty = true;
        self.is_connection_dirty = true;
        self.compute_region_centroids();
    }

    pub fn init_vbo_data(&mut self) {
        let count = self.system_count();

        self.system_positions = Vec::with_capacity(count * 3);
        for sys in &self.solar_systems {
            let [x, y, z] = sys.xyz();
            self.system_positions.push(x);
            self.system_positions.push(y);
            self.system_positions.push(z);
        }

        self.system_elements = (0..count as u32).collect();
        self.system_colors = vec![color_to_rgba32(DEFAULT_DRAW_COLOR); count];
        self.extract_connections();

        self.is_system_vbo_dirty = false;
        self.is_color_vao_dirty = false;
    }

    fn extract_connections(&mut self) {
        self.connection_positions.clear();
        self.connection_colors.clear();

        let regional_color: [f32; 4] = [0.25, 0.05, 0.05, 1.0];
        let normal_color: [f32; 4] = [0.1, 0.1, 0.5, 1.0];
        let count = self.system_count();

        for is_regional_pass in [true, false] {
            for sys_id in 0..count {
                let [sx, sy, sz] = self.solar_systems[sys_id].xyz();
                let num_conns = self.solar_systems[sys_id].connected_to.len();

                for ci in 0..num_conns {
                    let conn = self.solar_systems[sys_id].connected_to[ci];
                    if conn.to_system_id >= count {
                        continue;
                    }
                    if conn.is_regional != is_regional_pass {
                        continue;
                    }
                    let color = if conn.is_regional {
                        regional_color
                    } else {
                        normal_color
                    };

                    self.connection_positions.extend_from_slice(&[sx, sy, sz]);
                    self.connection_colors.extend_from_slice(&color);

                    let [tx, ty, tz] = self.solar_systems[conn.to_system_id].xyz();
                    self.connection_positions.extend_from_slice(&[tx, ty, tz]);
                    self.connection_colors.extend_from_slice(&color);
                }
            }
        }

        self.connection_vertex_count = self.connection_positions.len() / 3;
        self.is_connection_dirty = false;
        self.is_connection_vbo_dirty = true;
        log::info!("Extracted {} connection vertices ({} lines)", self.connection_vertex_count, self.connection_vertex_count / 2);
    }

    pub fn refresh_vbo_data(&mut self) {
        if self.is_system_vbo_dirty {
            for (i, sys) in self.solar_systems.iter().enumerate() {
                let [x, y, z] = sys.xyz();
                self.system_positions[i * 3] = x;
                self.system_positions[i * 3 + 1] = y;
                self.system_positions[i * 3 + 2] = z;
            }
            // update_vbos() will clear is_system_vbo_dirty after GPU upload
        }

        if self.is_connection_dirty {
            self.extract_connections();
        }

        let char_locs = &self.character_location_systems;
        for i in 0..self.system_count() {
            if char_locs.contains(&i) {
                if self.solar_systems[i].is_alerting {
                    self.system_colors[i] = color_to_rgba32(CHARACTER_ALERT_DRAW_COLOR);
                } else {
                    self.system_colors[i] = color_to_rgba32(CHARACTER_LOCATION_DRAW_COLOR);
                }
            } else {
                self.system_colors[i] = self.solar_systems[i].draw_color_argb32();
            }
        }
        self.is_color_vao_dirty = true;
    }

    pub fn set_current_home_system(&mut self, system_id: i32) {
        if self.home_system_id != -1 {
            self.clear_current_system();
        }
        self.home_system_id = system_id;
        if self.home_system_id == -1 {
            return;
        }
        self.add_green_crosshair(system_id as usize);

        for &red_id in self.red_crosshair_ids.iter() {
            self.pathfinding_queue.push_back(PathInfo {
                from_system: system_id as usize,
                to_system: red_id,
                ..Default::default()
            });
        }

        if self.home_system_id != -1 && self.character_location != -1 {
            self.pathfinding_queue.push_back(PathInfo {
                from_system: self.character_location as usize,
                to_system: system_id as usize,
                ..Default::default()
            });
        }
    }

    pub fn set_character_location(&mut self, system_id: i32) {
        self.character_location = system_id;
        if system_id == -1 {
            return;
        }
        let sid = system_id as usize;
        for &red_id in self.red_crosshair_ids.iter() {
            self.pathfinding_queue.push_back(PathInfo {
                from_system: sid,
                to_system: red_id,
                ..Default::default()
            });
        }
        if self.home_system_id != -1 {
            self.pathfinding_queue.push_back(PathInfo {
                from_system: sid,
                to_system: self.home_system_id as usize,
                ..Default::default()
            });
        }
    }

    pub fn clear_current_system(&mut self) {
        self.home_system_id = -1;
        self.green_crosshair_ids.clear();
    }

    pub fn set_character_location_systems(&mut self, system_ids: Vec<usize>) {
        self.character_location_systems = system_ids;
        self.is_color_vao_dirty = true;
    }

    pub fn incoming_tick(&mut self) -> bool {
        self.process_tick();
        !self.alert_systems.is_empty() || !self.highlight_systems.is_empty()
    }

    fn process_tick(&mut self) {
        let mut i = 0;
        while i < self.alert_systems.len() {
            let sys_id = self.alert_systems[i];
            let (_, atr) = self.solar_systems[sys_id].process_tick();
            if atr {
                self.solar_systems[sys_id].clear_alert();
                self.alert_systems.swap_remove(i);
                self.are_uniforms_clean = false;
            } else {
                i += 1;
            }
        }

        let mut i = 0;
        while i < self.highlight_systems.len() {
            let sys_id = self.highlight_systems[i];
            let (htr, _) = self.solar_systems[sys_id].process_tick();
            if htr {
                self.solar_systems[sys_id].clear_highlight();
                self.highlight_systems.swap_remove(i);
            } else {
                i += 1;
            }
        }

        if !self.alert_systems.is_empty() || !self.highlight_systems.is_empty() {
            self.are_uniforms_clean = false;
        }
    }

    pub fn add_alert(&mut self, system_id: usize, intel_report: Option<&str>) {
        if !self.alert_systems.contains(&system_id) {
            for &hl_id in &self.highlight_systems {
                self.solar_systems[hl_id].reset_highlight();
            }
            self.highlight_systems.clear();

            self.alert_systems.push(system_id);
            self.solar_systems[system_id].start_alert();

            let green_ids: Vec<usize> = self.green_crosshair_ids.iter().copied().collect();
            for green_id in green_ids {
                let cache_id = generate_unique_path_id(green_id, system_id);
                if !self.pathfinding_cache.contains_key(&cache_id) {
                    self.find_and_cache_path(green_id, system_id);
                }
            }

            self.are_uniforms_clean = false;
        }

        if let Some(stats) = self.system_stats.get_mut(&system_id) {
            stats.update(intel_report);
        } else {
            let mut stats = SystemStats::new();
            if let Some(report) = intel_report {
                stats.last_intel_report = report.to_string();
            }
            self.system_stats.insert(system_id, stats);
        }

        self.red_crosshair_ids.retain(|&x| x != system_id);
        self.red_crosshair_ids.push_back(system_id);

        while self.red_crosshair_ids.len() > self.max_alerts {
            if let Some(expired_id) = self.red_crosshair_ids.pop_front() {
                self.system_stats.remove(&expired_id);
            }
        }
    }

    pub fn add_green_crosshair(&mut self, system_id: usize) {
        if !self.green_crosshair_ids.contains(&system_id) {
            self.green_crosshair_ids.push_back(system_id);
        }
        while self.green_crosshair_ids.len() > 10 {
            self.green_crosshair_ids.pop_front();
        }
    }

    pub fn add_highlight(&mut self, system_id: usize, flash: bool) {
        if !self.highlight_systems.contains(&system_id) && !self.solar_systems[system_id].is_alerting
        {
            self.highlight_systems.push(system_id);
            self.solar_systems[system_id].start_highlight(flash);
            self.are_uniforms_clean = false;
        }
    }

    pub fn remove_expired_alerts(&mut self) {
        if self.max_alert_age == 0 {
            return;
        }
        let older_than = Utc::now() - Duration::minutes(self.max_alert_age as i64);
        let expired: Vec<usize> = self
            .system_stats
            .iter()
            .filter(|(_, stats)| !stats.expired && stats.last_report < older_than)
            .map(|(&id, _)| id)
            .collect();

        if !expired.is_empty() {
            self.red_crosshair_ids.retain(|id| !expired.contains(id));
            for id in &expired {
                self.system_stats.remove(id);
            }
        }
    }

    pub fn build_uniforms(&mut self) {
        self.uni_system_ids = [-1; 10];
        self.uni_colors = [[1.0, 1.0, 1.0, 1.0]; 10];
        self.uni_sizes = [0.0; 10];

        let total = (self.alert_systems.len() + self.highlight_systems.len()).min(10);
        let mut i = 0;

        if self.alert_systems.len() <= 10 {
            for &sys_id in &self.alert_systems {
                if i >= 10 {
                    break;
                }
                self.uni_system_ids[i] = sys_id as i32;
                self.uni_sizes[i] = self.solar_systems[sys_id].draw_size;
                self.uni_colors[i] = self.solar_systems[sys_id].draw_color_rgba_floats();
                i += 1;
            }
        } else {
            let skip = self.alert_systems.len() - 10;
            for (j, &sys_id) in self.alert_systems.iter().enumerate() {
                if j >= skip {
                    let idx = j - skip;
                    if idx < 10 {
                        self.uni_system_ids[idx] = sys_id as i32;
                        self.uni_sizes[idx] = self.solar_systems[sys_id].draw_size;
                        self.uni_colors[idx] = self.solar_systems[sys_id].draw_color_rgba_floats();
                    }
                }
            }
            i = self.alert_systems.len().min(10);
        }

        if i < total {
            for &sys_id in &self.highlight_systems {
                if i >= total {
                    break;
                }
                if self.alert_systems.contains(&sys_id) {
                    continue;
                }
                self.uni_system_ids[i] = sys_id as i32;
                self.uni_sizes[i] = self.solar_systems[sys_id].draw_size;
                self.uni_colors[i] = self.solar_systems[sys_id].draw_color_rgba_floats();
                i += 1;
            }
        }

        self.are_uniforms_clean = true;
    }

    pub fn find_path(&mut self, from: usize, to: usize) -> Option<PathInfo> {
        self.path_finder.as_mut().map(|pf| pf.find_path(from, to))
    }

    pub fn find_and_cache_path(&mut self, from: usize, to: usize) {
        let path_id = generate_unique_path_id(from, to);
        if !self.pathfinding_cache.contains_key(&path_id) {
            self.pathfinding_queue.push_back(PathInfo {
                from_system: from,
                to_system: to,
                ..Default::default()
            });
        }
    }

    pub fn process_pathfinding_queue(&mut self) {
        if !self.ok_to_process_paths || self.processing_path {
            return;
        }
        self.processing_path = true;

        if let Some(working) = self.pathfinding_queue.pop_front() {
            if let Some(result) = self.find_path(working.from_system, working.to_system) {
                let cache_id =
                    generate_unique_path_id(working.from_system, working.to_system);
                if !self.pathfinding_cache.contains_key(&cache_id) {
                    if self.pathfinding_cache.len() >= self.max_pathfinding_cache {
                        let keys: Vec<u64> = self.pathfinding_cache.keys()
                            .take(self.max_pathfinding_cache / 2)
                            .copied()
                            .collect();
                        for k in keys {
                            self.pathfinding_cache.remove(&k);
                        }
                    }
                    self.pathfinding_cache.insert(cache_id, result);
                }
            }
        }

        self.processing_path = false;
    }

}
