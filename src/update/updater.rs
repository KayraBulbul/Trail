use std::{
    env::consts::{ARCH, OS},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use chrono::{Local, NaiveDate};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

pub fn ignore_version(release: &Release) -> Result<(), Box<dyn std::error::Error>> {
    let (mut local_data, file_path) = load_local_data()?;
    local_data.ignored_version = Some(Version::parse(release.tag_name.trim_start_matches('v'))?);
    save_local_data(&local_data, &file_path)
}

pub fn install_update(release: Release) -> Result<(), Box<dyn std::error::Error>> {
    let ext = if OS == "windows" { "zip" } else { "tar.gz" };
    let expected = format!("trail-{OS}-{ARCH}.{ext}");

    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == expected)
        .ok_or_else(|| format!("No release build for {OS}-{ARCH}"))?;
    let expected_hash = expected_checksum(&release, &asset.name)?;

    // A unique directory keeps a leftover or concurrent update from colliding with this one.
    let temp_dir = std::env::temp_dir().join(format!("trail-update-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir)?;

    let result = download_and_replace(asset, &expected_hash, &temp_dir);
    let _ = fs::remove_dir_all(&temp_dir);
    result?;

    println!(
        "Updated Trail to {}. Restart trail to use it.",
        release.tag_name
    );
    Ok(())
}

fn download_and_replace(
    asset: &Asset,
    expected_hash: &str,
    temp_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading {}...", asset.name);
    let mut response = ureq::get(&asset.browser_download_url)
        .header("User-Agent", "trail")
        .call()?;
    let archive_path = temp_dir.join(&asset.name);
    let mut file = fs::File::create(&archive_path)?;
    io::copy(&mut response.body_mut().as_reader(), &mut file)?;
    drop(file);

    // Refuse to replace the running binary with a truncated or corrupted download.
    if sha256_file(&archive_path)? != expected_hash {
        return Err(format!("Checksum mismatch for {}, update aborted", asset.name).into());
    }

    let binary_path = extract_binary(&archive_path, temp_dir)?;
    self_replace::self_replace(&binary_path)?;
    Ok(())
}

fn expected_checksum(
    release: &Release,
    asset_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let sums = release
        .assets
        .iter()
        .find(|asset| asset.name == "SHA256SUMS.txt")
        .ok_or("Release is missing SHA256SUMS.txt")?;

    let content = ureq::get(&sums.browser_download_url)
        .header("User-Agent", "trail")
        .call()?
        .body_mut()
        .read_to_string()?;

    content
        .lines()
        .find_map(|line| {
            let (hash, name) = line.split_once(char::is_whitespace)?;
            (name.trim().trim_start_matches('*') == asset_name).then(|| hash.to_lowercase())
        })
        .ok_or_else(|| format!("SHA256SUMS.txt has no entry for {asset_name}").into())
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(not(windows))]
fn extract_binary(
    archive_path: &Path,
    temp_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let archive = fs::File::open(archive_path)?;
    tar::Archive::new(flate2::read::GzDecoder::new(archive)).unpack(temp_dir)?;
    find_binary(temp_dir, "trail")
}

#[cfg(windows)]
fn extract_binary(
    archive_path: &Path,
    temp_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let archive = fs::File::open(archive_path)?;
    zip::ZipArchive::new(archive)?.extract(temp_dir)?;
    find_binary(temp_dir, "trail.exe")
}

fn find_binary(temp_dir: &Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = temp_dir.join(name);
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("Release archive doesn't contain {name}").into())
    }
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
