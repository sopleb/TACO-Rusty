//! SDE Converter - Downloads the EVE Online Static Data Export and converts
//! mapSolarSystems + mapStargates + mapRegions into TACO's systemdata.json and regions.json.
//!
//! Usage: cargo run --bin sde_convert [-- --output-dir resources/data]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

const LATEST_URL: &str = "https://developers.eveonline.com/static-data/tranquility/latest.jsonl";
const SDE_URL_TEMPLATE: &str = "https://developers.eveonline.com/static-data/tranquility/eve-online-static-data-{BUILD}-jsonl.zip";
const COORD_SCALE: f64 = 1e14;

// --- SDE types ---

#[derive(Deserialize)]
struct SdeMeta {
    #[serde(rename = "buildNumber")]
    build_number: u64,
}

#[derive(Deserialize)]
struct SdePosition {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Deserialize)]
struct SdePosition2D {
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
struct SdeName {
    en: String,
}

#[derive(Deserialize)]
struct SdeSolarSystem {
    _key: u32,
    name: SdeName,
    position: SdePosition,
    #[serde(rename = "position2D")]
    position_2d: Option<SdePosition2D>,
    #[serde(rename = "regionID")]
    region_id: u32,
    #[allow(dead_code)]
    #[serde(rename = "stargateIDs")]
    stargate_ids: Option<Vec<u32>>,
}

#[derive(Deserialize)]
struct SdeStargateDestination {
    #[serde(rename = "solarSystemID")]
    solar_system_id: u32,
}

#[derive(Deserialize)]
struct SdeStargate {
    _key: u32,
    destination: SdeStargateDestination,
    #[serde(rename = "solarSystemID")]
    solar_system_id: u32,
}

#[derive(Deserialize)]
struct SdeRegion {
    _key: u32,
    name: SdeName,
}

// --- Output types ---

#[derive(Serialize)]
struct OutputConnection {
    to_system_id: usize,
    to_system_native_id: u32,
    is_regional: bool,
}

#[derive(Serialize)]
struct OutputSystem {
    id: usize,
    native_id: u32,
    name: String,
    x: f64,
    y: f64,
    z: f64,
    connected_to: Vec<OutputConnection>,
    x2d: f64,
    y2d: f64,
    region_id: u32,
}

fn main() {
    let output_dir = parse_output_dir();
    let version_path = output_dir.join("sde_version.txt");

    // Step 1: Get build number
    println!("Fetching latest SDE build info...");
    let meta_body = fetch_text(LATEST_URL);
    let meta: SdeMeta = serde_json::from_str(&meta_body).expect("Failed to parse latest.jsonl");
    println!("  Build: {}", meta.build_number);

    // Check if we already have this version
    if let Ok(local_version) = std::fs::read_to_string(&version_path) {
        if local_version.trim() == meta.build_number.to_string() {
            println!("SDE is already up to date (build {}).", meta.build_number);
            return;
        }
        println!("  Local build: {} -> updating", local_version.trim());
    }

    // Step 2: Download SDE zip
    let sde_url = SDE_URL_TEMPLATE.replace("{BUILD}", &meta.build_number.to_string());
    println!("Downloading SDE from {}...", sde_url);
    let zip_bytes = fetch_bytes(&sde_url);
    println!("  Downloaded {} MB", zip_bytes.len() / (1024 * 1024));

    // Step 3: Extract the three files we need
    println!("Extracting SDE files...");
    let mut solar_systems_raw = String::new();
    let mut stargates_raw = String::new();
    let mut regions_raw = String::new();

    {
        let cursor = Cursor::new(&zip_bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("Failed to open zip");
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let name = file.name().to_string();
            match name.as_str() {
                "mapSolarSystems.jsonl" => {
                    file.read_to_string(&mut solar_systems_raw).unwrap();
                }
                "mapStargates.jsonl" => {
                    file.read_to_string(&mut stargates_raw).unwrap();
                }
                "mapRegions.jsonl" => {
                    file.read_to_string(&mut regions_raw).unwrap();
                }
                _ => {}
            }
        }
    }

    assert!(!solar_systems_raw.is_empty(), "mapSolarSystems.jsonl not found in SDE zip");
    assert!(!stargates_raw.is_empty(), "mapStargates.jsonl not found in SDE zip");
    assert!(!regions_raw.is_empty(), "mapRegions.jsonl not found in SDE zip");

    // Step 4: Parse solar systems (k-space only: 30000000-30999999)
    println!("Parsing solar systems...");
    let mut sde_systems: Vec<SdeSolarSystem> = Vec::new();
    for line in solar_systems_raw.lines() {
        if line.is_empty() { continue; }
        if let Ok(sys) = serde_json::from_str::<SdeSolarSystem>(line) {
            if (30_000_000..31_000_000).contains(&sys._key) {
                sde_systems.push(sys);
            }
        }
    }
    // Sort by native_id for stable ordering
    sde_systems.sort_by_key(|s| s._key);
    println!("  {} k-space systems", sde_systems.len());

    // Build native_id -> sequential index mapping
    let native_to_idx: HashMap<u32, usize> = sde_systems
        .iter()
        .enumerate()
        .map(|(i, s)| (s._key, i))
        .collect();

    // Build native_id -> region_id mapping
    let native_to_region: HashMap<u32, u32> = sde_systems
        .iter()
        .map(|s| (s._key, s.region_id))
        .collect();

    // Step 5: Parse stargates and build connection map
    println!("Parsing stargates...");
    let mut connections: HashMap<u32, Vec<(u32, bool)>> = HashMap::new(); // source_native -> [(dest_native, is_regional)]

    for line in stargates_raw.lines() {
        if line.is_empty() { continue; }
        if let Ok(gate) = serde_json::from_str::<SdeStargate>(line) {
            let src = gate.solar_system_id;
            let dst = gate.destination.solar_system_id;
            // Only k-space connections
            if !native_to_idx.contains_key(&src) || !native_to_idx.contains_key(&dst) {
                continue;
            }
            let src_region = native_to_region.get(&src).copied().unwrap_or(0);
            let dst_region = native_to_region.get(&dst).copied().unwrap_or(0);
            let is_regional = src_region != dst_region;

            connections
                .entry(src)
                .or_default()
                .push((dst, is_regional));
        }
    }

    let total_gates: usize = connections.values().map(|v| v.len()).sum();
    println!("  {} stargate connections", total_gates);

    // Step 6: Build output systems
    println!("Building systemdata.json...");
    let output_systems: Vec<OutputSystem> = sde_systems
        .iter()
        .enumerate()
        .map(|(idx, sys)| {
            let conns = connections
                .get(&sys._key)
                .map(|c| {
                    c.iter()
                        .filter_map(|&(dst_native, is_regional)| {
                            native_to_idx.get(&dst_native).map(|&dst_idx| OutputConnection {
                                to_system_id: dst_idx,
                                to_system_native_id: dst_native,
                                is_regional,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            let (x2d, y2d) = match &sys.position_2d {
                Some(p) => (p.x / COORD_SCALE, p.y / COORD_SCALE),
                None => (sys.position.x / COORD_SCALE, sys.position.z / COORD_SCALE),
            };

            OutputSystem {
                id: idx,
                native_id: sys._key,
                name: sys.name.en.clone(),
                x: sys.position.x / COORD_SCALE,
                y: sys.position.z / COORD_SCALE,  // SDE Z -> map Y
                z: sys.position.y / COORD_SCALE,  // SDE Y -> map Z (vertical height)
                connected_to: conns,
                x2d,
                y2d,
                region_id: sys.region_id,
            }
        })
        .collect();

    // Step 7: Build regions.json
    println!("Building regions.json...");
    let mut region_map: HashMap<String, String> = HashMap::new();
    for line in regions_raw.lines() {
        if line.is_empty() { continue; }
        if let Ok(region) = serde_json::from_str::<SdeRegion>(line) {
            // Only include regions that have k-space systems
            if sde_systems.iter().any(|s| s.region_id == region._key) {
                region_map.insert(region._key.to_string(), region.name.en.clone());
            }
        }
    }
    println!("  {} regions", region_map.len());

    // Step 8: Write output
    let systemdata_path = output_dir.join("systemdata.json");
    let regions_path = output_dir.join("regions.json");

    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let systemdata_json = serde_json::to_string(&output_systems).expect("Failed to serialize systemdata");
    std::fs::write(&systemdata_path, &systemdata_json).expect("Failed to write systemdata.json");
    println!("Wrote {} ({} systems, {} bytes)", systemdata_path.display(), output_systems.len(), systemdata_json.len());

    let regions_json = serde_json::to_string_pretty(&region_map).expect("Failed to serialize regions");
    std::fs::write(&regions_path, &regions_json).expect("Failed to write regions.json");
    println!("Wrote {} ({} regions)", regions_path.display(), region_map.len());

    std::fs::write(&version_path, meta.build_number.to_string())
        .expect("Failed to write sde_version.txt");
    println!("Done! SDE build {} converted successfully.", meta.build_number);
}

fn parse_output_dir() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--output-dir" && i + 1 < args.len() {
            return PathBuf::from(&args[i + 1]);
        }
    }
    PathBuf::from("resources/data")
}

fn fetch_text(url: &str) -> String {
    let response = ureq::get(url)
        .call()
        .expect("HTTP request failed");
    let mut body = String::new();
    response.into_body()
        .as_reader()
        .read_to_string(&mut body)
        .expect("Failed to read response body");
    body
}

fn fetch_bytes(url: &str) -> Vec<u8> {
    let mut body = Vec::new();
    ureq::get(url)
        .call()
        .expect("HTTP request failed")
        .into_body()
        .as_reader()
        .read_to_end(&mut body)
        .expect("Failed to read response body");
    body
}
