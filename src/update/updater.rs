use std::{fs, io, path::PathBuf};

use chrono::{Local, NaiveDate};
use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Default, Serialize, Deserialize)]
struct LocalData {
    last_checked: NaiveDate,
    ignored_version: Option<Version>,
    latest_version: Option<Version>,
}

static GH_URL: &str = "https://api.github.com/repos/KayraBulbul/Trail/releases/latest";

pub fn need_update() -> Result<Option<Release>, Box<dyn std::error::Error>> {
    let (mut local_data, file_path) = load_local_data()?;

    let today = Local::now().date_naive();
    if today != local_data.last_checked {
        let body = ureq::get(GH_URL)
            .header("User-Agent", "trail")
            .call()?
            .body_mut()
            .read_json::<Release>()?;

        let latest = Version::parse(body.tag_name.trim_start_matches('v'))?;
        let local = Version::parse(env!("CARGO_PKG_VERSION"))?;

        local_data.last_checked = today;
        local_data.latest_version = Some(latest.clone());
        save_local_data(&local_data, &file_path)?;

        if local_data.ignored_version.as_ref() == Some(&latest) {
            return Ok(None);
        }

        if latest > local {
            return Ok(Some(body));
        }
    }
    Ok(None)
}

fn load_local_data() -> Result<(LocalData, PathBuf), Box<dyn std::error::Error>> {
    let data_dir = dirs::data_local_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Unable to find local data directory.",
        )
    })?;

    let trail_dir = data_dir.join("trail");
    std::fs::create_dir_all(&trail_dir)?;

    let file_path = trail_dir.join("trail.json");
    let content = fs::read_to_string(&file_path)?;

    let local_data: LocalData = match fs::read_to_string(&file_path) {
        Ok(content) => serde_json::from_str(&content)?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => LocalData::default(),
        Err(e) => return Err(e.into()),
    };

    Ok((local_data, file_path))
}

fn save_local_data(data: &LocalData, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, serde_json::to_string_pretty(data)?)?;
    Ok(())
}
