#[derive(Clone, Debug, Default)]
pub struct PathInfo {
    pub total_jumps: i32,
    pub from_system: usize,
    pub to_system: usize,
}

pub fn generate_unique_path_id(from_system: usize, to_system: usize) -> u64 {
    (from_system as u64) * 100_000 + (to_system as u64)
}
