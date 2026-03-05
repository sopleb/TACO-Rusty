use std::collections::BinaryHeap;
use std::cmp::Ordering;

use crate::core::path_info::PathInfo;
use crate::core::solar_system::SolarSystemConnection;

#[derive(Debug)]
struct Node {
    cost: u32,
    system_id: usize,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}
impl Eq for Node {}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
    }
}

pub struct SolarSystemPathFinder {
    size: usize,
    is_blocked: Vec<bool>,
    connections: Vec<Vec<SolarSystemConnection>>,
    visited: Vec<bool>,
    heap: BinaryHeap<Node>,
}

impl SolarSystemPathFinder {
    pub fn new(size: usize, connections: Vec<Vec<SolarSystemConnection>>) -> Self {
        Self {
            is_blocked: vec![false; size],
            visited: vec![false; size],
            heap: BinaryHeap::with_capacity(256),
            size,
            connections,
        }
    }

    pub fn find_path(&mut self, start: usize, end: usize) -> PathInfo {
        self.find_path_reversed(end, start)
    }

    fn find_path_reversed(&mut self, start: usize, end: usize) -> PathInfo {
        self.visited.iter_mut().for_each(|v| *v = false);
        self.visited[start] = true;
        self.heap.clear();

        self.heap.push(Node {
            cost: 0,
            system_id: start,
        });

        while let Some(node) = self.heap.pop() {
            if node.system_id == end {
                return PathInfo {
                    total_jumps: node.cost as i32,
                    from_system: start,
                    to_system: end,
                };
            }

            if node.system_id < self.connections.len() {
                for conn in &self.connections[node.system_id] {
                    let temp_id = conn.to_system_id;
                    if temp_id >= self.size {
                        continue;
                    }
                    if self.is_blocked[temp_id] || self.visited[temp_id] {
                        continue;
                    }
                    self.visited[temp_id] = true;
                    self.heap.push(Node {
                        cost: node.cost + 1,
                        system_id: temp_id,
                    });
                }
            }
        }

        PathInfo {
            total_jumps: -1,
            from_system: start,
            to_system: end,
        }
    }
}
