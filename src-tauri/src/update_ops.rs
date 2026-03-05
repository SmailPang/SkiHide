use std::{collections::HashMap, path::PathBuf};

use futures_util::StreamExt;
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};

use crate::models::{
    AppConfig, MirrorCdkValidationInfo, MirrorDownloadInfo, UpdateCheckInfo, UpdateDownloadResult,
};

pub const UPDATE_DOWNLOAD_PROGRESS_EVENT: &str = "skihide://update-download-progress";

const MIRROR_ENDPOINT: &str = "https://mirrorchyan.com/api/resources/SkiHide/latest";
const SKIHIDE_ENDPOINT: &str = "https://update.skihide.xyz/api";
const RAINYUN_CDN_BASE: &str = "https://skihide.cn-nb1.rains3.com";
const GITHUB_RELEASE_BASE: &str = "https://github.com/SmailPang/SkiHide/releases/download";

#[derive(Deserialize)]
struct MirrorResponse {
    code: i32,
    msg: String,
    data: Option<MirrorData>,
}

#[derive(Deserialize)]
struct MirrorData {
    version_name: String,
    release_note: Option<String>,
    url: Option<String>,
    sha256: Option<String>,
}

#[derive(Deserialize)]
struct SkiHideResponse {
    version: String,
    #[allow(dead_code)]
    build: Option<i64>,
    update_log: String,
    download: HashMap<String, SkiHideDownloadEntry>,
    sha256: Option<String>,
}

#[derive(Deserialize)]
struct SkiHideDownloadEntry {
    url: String,
}

pub async fn check_for_updates(
    current_version: &str,
    config: &AppConfig,
) -> Result<UpdateCheckInfo, String> {
    let client = Client::new();
    let normalized_current = normalize_version(current_version)?;

    if config.update_source == "mirror_chan" {
        let query: Vec<(&str, String)> = vec![
            ("current_version", current_version.to_string()),
            ("user_agent", "skihide-client".to_string()),
        ];
        let mirror_resp = client
            .get(MIRROR_ENDPOINT)
            .query(&query)
            .send()
            .await
            .map_err(|error| format!("failed to request mirror update info: {error}"))?;

        let mirror_data: MirrorResponse = mirror_resp
            .json()
            .await
            .map_err(|error| format!("failed to parse mirror response: {error}"))?;

        if mirror_data.code != 0 {
            return Ok(UpdateCheckInfo {
                source: "mirror_chan".to_string(),
                current_version: current_version.to_string(),
                latest_version: current_version.to_string(),
                changelog: String::new(),
                has_update: false,
                download_url: None,
                download_candidates: Vec::new(),
                sha256: None,
                mirror_code: Some(mirror_data.code),
                mirror_message: Some(mirror_data.msg),
            });
        }

        let Some(data) = mirror_data.data else {
            return Err("mirror response missing data field".to_string());
        };

        let normalized_latest = normalize_version(&data.version_name)?;
        let has_update = normalized_latest > normalized_current;
        let mut download_url = data.url.clone();
        let mut download_candidates = Vec::new();
        let mut sha256 = data.sha256.clone();

        if has_update && download_url.is_none() {
            let can_use_mirror_download =
                config.download_source == "mirror_chan" && !config.mirror_chan_sdk.trim().is_empty();
            if !can_use_mirror_download {
                let official = fetch_skihide_info(&client, &config.language).await?;
                download_candidates = build_download_candidates(
                    &official.download,
                    &config.download_source,
                    &data.version_name,
                );
                download_url = download_candidates.first().cloned();
                if sha256.is_none() {
                    sha256 = official.sha256.clone();
                }
            }
        } else if let Some(url) = &download_url {
            download_candidates.push(url.clone());
        }

        return Ok(UpdateCheckInfo {
            source: "mirror_chan".to_string(),
            current_version: current_version.to_string(),
            latest_version: data.version_name,
            changelog: data.release_note.unwrap_or_default(),
            has_update,
            download_url,
            download_candidates,
            sha256,
            mirror_code: None,
            mirror_message: None,
        });
    }

    let official = fetch_skihide_info(&client, &config.language).await?;
    let normalized_latest = normalize_version(&official.version)?;
    let has_update = normalized_latest > normalized_current;
    let download_candidates = if has_update {
        build_download_candidates(&official.download, &config.download_source, &official.version)
    } else {
        Vec::new()
    };
    let download_url = download_candidates.first().cloned();

    Ok(UpdateCheckInfo {
        source: "skihide".to_string(),
        current_version: current_version.to_string(),
        latest_version: official.version,
        changelog: official.update_log,
        has_update,
        download_url,
        download_candidates,
        sha256: official.sha256,
        mirror_code: None,
        mirror_message: None,
    })
}

pub async fn resolve_mirror_download_with_cdk(
    current_version: &str,
    cdk: &str,
) -> Result<MirrorDownloadInfo, String> {
    let cdk = cdk.trim();
    if cdk.is_empty() {
        return Ok(MirrorDownloadInfo {
            url: None,
            sha256: None,
            mirror_code: Some(7002),
            mirror_message: Some("cdk is empty".to_string()),
        });
    }

    let client = Client::new();
    let query: Vec<(&str, String)> = vec![
        ("current_version", current_version.to_string()),
        ("user_agent", "skihide-client".to_string()),
        ("cdk", cdk.to_string()),
    ];

    let mirror_resp = client
        .get(MIRROR_ENDPOINT)
        .query(&query)
        .send()
        .await
        .map_err(|error| format!("failed to request mirror download url: {error}"))?;

    let mirror_data: MirrorResponse = mirror_resp
        .json()
        .await
        .map_err(|error| format!("failed to parse mirror download response: {error}"))?;

    if mirror_data.code != 0 {
        return Ok(MirrorDownloadInfo {
            url: None,
            sha256: None,
            mirror_code: Some(mirror_data.code),
            mirror_message: Some(mirror_data.msg),
        });
    }

    let url = mirror_data
        .data
        .as_ref()
        .and_then(|data| data.url.clone());
    let sha256 = mirror_data
        .data
        .as_ref()
        .and_then(|data| data.sha256.clone());

    Ok(MirrorDownloadInfo {
        url,
        sha256,
        mirror_code: None,
        mirror_message: None,
    })
}

pub async fn validate_mirror_cdk(
    current_version: &str,
    cdk: &str,
) -> Result<MirrorCdkValidationInfo, String> {
    let cdk = cdk.trim();
    if cdk.is_empty() {
        return Ok(MirrorCdkValidationInfo {
            valid: false,
            mirror_code: Some(7002),
            mirror_message: Some("cdk is empty".to_string()),
        });
    }

    let client = Client::new();
    let query: Vec<(&str, String)> = vec![
        ("current_version", current_version.to_string()),
        ("user_agent", "skihide-client".to_string()),
        ("cdk", cdk.to_string()),
    ];

    let mirror_resp = client
        .get(MIRROR_ENDPOINT)
        .query(&query)
        .send()
        .await
        .map_err(|error| format!("failed to validate mirror cdk: {error}"))?;

    let mirror_data: MirrorResponse = mirror_resp
        .json()
        .await
        .map_err(|error| format!("failed to parse mirror cdk validation response: {error}"))?;

    if mirror_data.code != 0 {
        return Ok(MirrorCdkValidationInfo {
            valid: false,
            mirror_code: Some(mirror_data.code),
            mirror_message: Some(mirror_data.msg),
        });
    }

    Ok(MirrorCdkValidationInfo {
        valid: true,
        mirror_code: None,
        mirror_message: None,
    })
}

pub async fn download_update_with_fallback(
    app: &AppHandle,
    urls: &[String],
    expected_sha256: Option<&str>,
    version: &str,
) -> Result<UpdateDownloadResult, String> {
    if urls.is_empty() {
        return Err("no download url available".to_string());
    }

    let mut errors = Vec::new();
    for (idx, url) in urls.iter().enumerate() {
        match download_update_package_once(app, url, expected_sha256, version).await {
            Ok((file_path, actual_sha256)) => {
                return Ok(UpdateDownloadResult {
                    file_path: file_path.to_string_lossy().to_string(),
                    sha256: Some(actual_sha256),
                    used_url: url.clone(),
                    fallback_used: idx > 0,
                });
            }
            Err(error) => errors.push(format!("{url} -> {error}")),
        }
    }

    Err(format!(
        "all download sources failed: {}",
        errors.join(" | ")
    ))
}

async fn download_update_package_once(
    app: &AppHandle,
    url: &str,
    expected_sha256: Option<&str>,
    version: &str,
) -> Result<(PathBuf, String), String> {
    let client = Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("failed to request update package: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "download request failed with status {}",
            response.status()
        ));
    }

    let total_size = response.content_length();
    let updates_dir = resolve_updates_dir()?;
    fs::create_dir_all(&updates_dir)
        .await
        .map_err(|error| format!("failed to create updates directory: {error}"))?;

    let file_name = format!("SkiHide-{}.exe", sanitize_version(version));
    let file_path = updates_dir.join(file_name);
    let mut file = File::create(&file_path)
        .await
        .map_err(|error| format!("failed to create update file: {error}"))?;

    let mut downloaded = 0u64;
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("failed while downloading update: {error}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|error| format!("failed writing update file: {error}"))?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;

        if let Some(total) = total_size {
            if total > 0 {
                let percent = ((downloaded as f64 / total as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u8;
                let _ = app.emit(UPDATE_DOWNLOAD_PROGRESS_EVENT, percent);
            }
        }
    }

    file.flush()
        .await
        .map_err(|error| format!("failed flushing update file: {error}"))?;

    let actual_sha256 = format!("{:x}", hasher.finalize());
    if let Some(expected) = expected_sha256 {
        let expected = expected.trim().to_ascii_lowercase();
        if !expected.is_empty() && expected != actual_sha256 {
            let _ = fs::remove_file(&file_path).await;
            return Err("sha256 verification failed for downloaded package".to_string());
        }
    }

    let _ = app.emit(UPDATE_DOWNLOAD_PROGRESS_EVENT, 100u8);

    Ok((file_path, actual_sha256))
}

async fn fetch_skihide_info(client: &Client, language: &str) -> Result<SkiHideResponse, String> {
    let response = client
        .get(SKIHIDE_ENDPOINT)
        .query(&[("lang", language)])
        .send()
        .await
        .map_err(|error| format!("failed to request skihide update info: {error}"))?;

    response
        .json::<SkiHideResponse>()
        .await
        .map_err(|error| format!("failed to parse skihide response: {error}"))
}

fn build_download_candidates(
    download_map: &HashMap<String, SkiHideDownloadEntry>,
    source: &str,
    latest_version: &str,
) -> Vec<String> {
    let version = latest_version.trim();
    if version.is_empty() {
        return Vec::new();
    }

    let rainyun_url = build_rainyun_cdn_url(version);
    let github_url = build_github_url(version);

    let mut result = if source == "github" {
        vec![github_url, rainyun_url]
    } else {
        vec![rainyun_url, github_url]
    };

    if let Some(url) = find_official_download(download_map, "rainyun_cdn") {
        if !result.iter().any(|item| item == &url) {
            result.push(url);
        }
    }
    if let Some(url) = find_official_download(download_map, "github") {
        if !result.iter().any(|item| item == &url) {
            result.push(url);
        }
    }

    result
}

fn build_rainyun_cdn_url(version: &str) -> String {
    format!("{RAINYUN_CDN_BASE}/SkiHide-{version}.exe")
}

fn build_github_url(version: &str) -> String {
    format!("{GITHUB_RELEASE_BASE}/{version}/SkiHide-{version}.exe")
}

fn find_official_download(
    download_map: &HashMap<String, SkiHideDownloadEntry>,
    key: &str,
) -> Option<String> {
    let normalize = |value: &str| {
        value
            .to_ascii_lowercase()
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>()
    };
    let expected = normalize(key);

    download_map
        .iter()
        .find(|(entry_key, _)| normalize(entry_key) == expected)
        .map(|(_, entry)| entry.url.clone())
}

fn resolve_updates_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| format!("failed to resolve exe path: {error}"))?;
    let base = exe
        .parent()
        .ok_or_else(|| "failed to resolve executable directory".to_string())?;
    Ok(base.join("updates"))
}

fn normalize_version(raw: &str) -> Result<Version, String> {
    let normalized = raw.trim().trim_start_matches('v').trim_start_matches('V');
    Version::parse(normalized).map_err(|error| format!("invalid version `{raw}`: {error}"))
}

fn sanitize_version(version: &str) -> String {
    version
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
