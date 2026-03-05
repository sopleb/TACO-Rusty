use std::io::Read;
use std::path::Path;

const LATEST_URL: &str =
    "https://developers.eveonline.com/static-data/tranquility/latest.jsonl";

fn main() {
    println!("cargo:rerun-if-changed=resources/data/sde_version.txt");
    println!("cargo:rerun-if-changed=resources/data/systemdata.json");
    println!("cargo:rerun-if-changed=resources/icons/app.ico");

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("resources/icons/app.ico");
        res.set("ProductName", "T.A.C.O.");
        res.set("FileDescription", "Tactical Awareness Control Overlay for EVE Online");
        res.set("CompanyName", "sopleb");
        res.set("LegalCopyright", "Copyright (c) 2025 sopleb");
        res.set("OriginalFilename", "taco.exe");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resources: {e}");
        }
    }

    let data_dir = Path::new("resources/data");
    let version_path = data_dir.join("sde_version.txt");
    let systemdata_path = data_dir.join("systemdata.json");

    if !systemdata_path.exists() {
        println!(
            "cargo:warning=SDE data not found! Run first: cargo run --bin sde_convert"
        );
        return;
    }

    let local_version = std::fs::read_to_string(&version_path)
        .unwrap_or_default()
        .trim()
        .to_string();

    if local_version.is_empty() {
        println!(
            "cargo:warning=No SDE version file found. Run: cargo run --bin sde_convert"
        );
        return;
    }

    let remote_version = match fetch_build_number() {
        Some(v) => v,
        None => return,
    };

    if local_version != remote_version {
        println!(
            "cargo:warning=SDE is outdated (local: {}, remote: {}). Run: cargo run --bin sde_convert",
            local_version, remote_version
        );
    }
}

fn fetch_build_number() -> Option<String> {
    let response = ureq::get(LATEST_URL).call().ok()?;
    let mut body = String::new();
    response
        .into_body()
        .as_reader()
        .take(4096)
        .read_to_string(&mut body)
        .ok()?;

    let parsed: serde_json::Value = serde_json::from_str(&body).ok()?;
    parsed
        .get("buildNumber")
        .and_then(|v| v.as_u64())
        .map(|n| n.to_string())
}
