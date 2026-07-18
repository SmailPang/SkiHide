use std::path::PathBuf;

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
const CLOUDFLARE_ENDPOINT: &str = "https://v2.skihide.xyz";
const CNB_RELEASE_BASE: &str = "https://cnb.cool/SmailPang/SkiHide/-/releases/download";
const GITHUB_RELEASE_BASE: &str = "https://github.com/SmailPang/SkiHide/releases/download";
const SOFTWARE_NAME: &str = "SkiHide";

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
    sha256: Option<String>,
}

#[derive(Deserialize)]
struct CloudflareResponse {
    version: String,
    changelog: String,
    downloads: Vec<CloudflareDownload>,
}

#[derive(Deserialize)]
struct CloudflareDownload {
    name: String,
    download_url: String,
    digest: Option<String>,
}

pub async fn check_for_updates(
    current_version: &str,
    config: &AppConfig,
) -> Result<UpdateCheckInfo, String> {
    let client = update_client()?;
    if config.update_source == "cloudflare" {
        let channel = cloudflare_channel(&config.update_channel);
        let cloudflare = fetch_cloudflare_info(&client, current_version, Some(channel)).await?;
        let has_update = has_newer_version(current_version, &cloudflare.version)?;
        let cloudflare_asset = select_cloudflare_asset(&cloudflare);
        let download_candidates = if has_update {
            build_download_candidates(
                &config.download_source,
                &cloudflare.version,
                cloudflare_asset.map(|asset| asset.download_url.as_str()),
            )
        } else {
            Vec::new()
        };
        let sha256 = cloudflare_asset
            .and_then(|asset| asset.digest.as_deref())
            .and_then(normalize_sha256);

        return Ok(UpdateCheckInfo {
            source: "cloudflare".to_string(),
            current_version: current_version.to_string(),
            latest_version: cloudflare.version,
            changelog: cloudflare.changelog,
            has_update,
            download_url: download_candidates.first().cloned(),
            download_candidates,
            sha256,
            mirror_code: None,
            mirror_message: None,
        });
    }

    if config.update_source == "mirror_chan" {
        let channel = match config.update_channel.as_str() {
            "beta" => "beta",
            _ => "stable",
        };
        let query: Vec<(&str, String)> = vec![
            ("current_version", current_version.to_string()),
            ("user_agent", "skihide-client".to_string()),
            ("channel", channel.to_string()),
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

        let has_update = has_newer_version(current_version, &data.version_name)?;
        let mut download_url = data.url.clone();
        let mut download_candidates = Vec::new();
        let mut sha256 = data.sha256.clone();

        if has_update && download_url.is_none() {
            let can_use_mirror_download = config.download_source == "mirror_chan"
                && !config.mirror_chan_sdk.trim().is_empty();
            if !can_use_mirror_download {
                let official = fetch_skihide_info(&client, &config.language).await?;
                let cloudflare = fetch_cloudflare_for_download_source(
                    &client,
                    &config.download_source,
                    &data.version_name,
                )
                .await;
                download_candidates = build_download_candidates(
                    &config.download_source,
                    &data.version_name,
                    cloudflare.as_deref(),
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
    let has_update = has_newer_version(current_version, &official.version)?;
    let download_candidates = if has_update {
        let cloudflare = fetch_cloudflare_for_download_source(
            &client,
            &config.download_source,
            &official.version,
        )
        .await;
        build_download_candidates(
            &config.download_source,
            &official.version,
            cloudflare.as_deref(),
        )
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
    channel: &str,
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

    let client = update_client()?;
    let query: Vec<(&str, String)> = vec![
        ("current_version", current_version.to_string()),
        ("user_agent", "skihide-client".to_string()),
        ("cdk", cdk.to_string()),
        ("channel", channel.to_string()),
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

    let url = mirror_data.data.as_ref().and_then(|data| data.url.clone());
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
    channel: &str,
) -> Result<MirrorCdkValidationInfo, String> {
    let cdk = cdk.trim();
    if cdk.is_empty() {
        return Ok(MirrorCdkValidationInfo {
            valid: false,
            mirror_code: Some(7002),
            mirror_message: Some("cdk is empty".to_string()),
        });
    }

    let client = update_client()?;
    let query: Vec<(&str, String)> = vec![
        ("current_version", current_version.to_string()),
        ("user_agent", "skihide-client".to_string()),
        ("cdk", cdk.to_string()),
        ("channel", channel.to_string()),
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
    let client = update_client()?;
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
        if crate::is_app_exiting(app) {
            let _ = fs::remove_file(&file_path).await;
            return Err("download cancelled because application is shutting down".to_string());
        }

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
                emit_download_progress(app, percent);
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

    emit_download_progress(app, 100);

    Ok((file_path, actual_sha256))
}

fn emit_download_progress(app: &AppHandle, percent: u8) {
    if crate::is_app_exiting(app) {
        return;
    }
    let _ = app.emit(UPDATE_DOWNLOAD_PROGRESS_EVENT, percent);
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

async fn fetch_cloudflare_info(
    client: &Client,
    version: &str,
    channel: Option<&str>,
) -> Result<CloudflareResponse, String> {
    let mut request = client
        .get(format!("{CLOUDFLARE_ENDPOINT}/api/check-update"))
        .query(&[("version", version)]);
    if let Some(channel) = channel {
        request = request.query(&[("channel", channel)]);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("failed to request Cloudflare update info: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Cloudflare update request failed with status {}",
            response.status()
        ));
    }

    response
        .json::<CloudflareResponse>()
        .await
        .map_err(|error| format!("failed to parse Cloudflare response: {error}"))
}

async fn fetch_cloudflare_for_download_source(
    client: &Client,
    source: &str,
    version: &str,
) -> Option<String> {
    if source != "cloudflare" {
        return None;
    }

    fetch_cloudflare_info(client, version, None)
        .await
        .ok()
        .and_then(|response| {
            select_cloudflare_asset(&response).map(|asset| asset.download_url.clone())
        })
}

fn select_cloudflare_asset(response: &CloudflareResponse) -> Option<&CloudflareDownload> {
    let expected_name = format!("SkiHide-{}.exe", response.version);
    response
        .downloads
        .iter()
        .find(|asset| asset.name.eq_ignore_ascii_case(&expected_name))
        .or_else(|| {
            response
                .downloads
                .iter()
                .find(|asset| asset.name.ends_with(".exe"))
        })
}

fn normalize_sha256(digest: &str) -> Option<String> {
    let value = digest.trim();
    let value = value.strip_prefix("sha256:").unwrap_or(value);
    (!value.is_empty()).then(|| value.to_ascii_lowercase())
}

fn update_client() -> Result<Client, String> {
    Client::builder()
        .user_agent(update_user_agent())
        .build()
        .map_err(|error| format!("failed to create update client: {error}"))
}

fn update_user_agent() -> String {
    format!("{SOFTWARE_NAME}/{}", env!("CARGO_PKG_VERSION"))
}

fn cloudflare_channel(update_channel: &str) -> &'static str {
    match update_channel {
        "beta" => "prerelease",
        _ => "release",
    }
}

fn build_download_candidates(
    source: &str,
    latest_version: &str,
    cloudflare_url: Option<&str>,
) -> Vec<String> {
    let version = latest_version.trim();
    if version.is_empty() {
        return Vec::new();
    }

    let cnb_url = build_cnb_url(version);
    let github_url = build_github_url(version);

    match source {
        "cloudflare" => cloudflare_url
            .filter(|url| !url.trim().is_empty())
            .map(|url| vec![url.to_string(), cnb_url.clone(), github_url.clone()])
            .unwrap_or_else(|| vec![cnb_url, github_url]),
        "github" => vec![github_url, cnb_url],
        _ => vec![cnb_url, github_url],
    }
}

fn build_cnb_url(version: &str) -> String {
    format!("{CNB_RELEASE_BASE}/{version}/SkiHide-{version}.exe")
}

fn build_github_url(version: &str) -> String {
    format!("{GITHUB_RELEASE_BASE}/{version}/SkiHide-{version}.exe")
}

fn resolve_updates_dir() -> Result<PathBuf, String> {
    let exe =
        std::env::current_exe().map_err(|error| format!("failed to resolve exe path: {error}"))?;
    let base = exe
        .parent()
        .ok_or_else(|| "failed to resolve executable directory".to_string())?;
    Ok(base.join("updates"))
}

/// Canonicalize SkiHide version strings so `2.0.1-Beta.2` and `2.0.1-beta2` compare equal.
fn canonicalize_version_tag(raw: &str) -> String {
    let trimmed = raw.trim().trim_start_matches('v').trim_start_matches('V');
    let lower = trimmed.to_ascii_lowercase();

    let Some((core, prerelease)) = lower.split_once('-') else {
        return lower;
    };

    let core = core.trim();
    let prerelease = normalize_prerelease_tag(prerelease.trim());
    if prerelease.is_empty() {
        return core.to_string();
    }

    format!("{core}-{prerelease}")
}

fn normalize_prerelease_tag(pre: &str) -> String {
    if let Some(rest) = pre.strip_prefix("beta") {
        let rest = rest.trim_start_matches('.').trim_start_matches('_');
        if rest.is_empty() {
            return "beta".to_string();
        }
        let digits: String = rest.chars().filter(|ch| ch.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return format!("beta.{digits}");
        }
    }

    pre.to_string()
}

fn normalize_version(raw: &str) -> Result<Version, String> {
    let canonical = canonicalize_version_tag(raw);
    Version::parse(&canonical).map_err(|error| format!("invalid version `{raw}`: {error}"))
}

fn has_newer_version(current: &str, latest: &str) -> Result<bool, String> {
    let current_v = normalize_version(current)?;
    let latest_v = normalize_version(latest)?;
    Ok(latest_v > current_v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beta_tags_with_different_format_are_equal() {
        assert_eq!(
            normalize_version("2.0.1-Beta.2").unwrap(),
            normalize_version("2.0.1-beta2").unwrap()
        );
        assert!(!has_newer_version("2.0.1-Beta.2", "2.0.1-beta2").unwrap());
        assert!(!has_newer_version("2.0.1-beta2", "2.0.1-Beta.2").unwrap());
    }

    #[test]
    fn stable_release_is_not_older_than_beta_prerelease() {
        assert!(!has_newer_version("2.0.1", "2.0.1-beta2").unwrap());
    }

    #[test]
    fn newer_beta_build_is_detected() {
        assert!(has_newer_version("2.0.1-beta1", "2.0.1-beta2").unwrap());
    }

    #[test]
    fn cnb_download_url_uses_release_version() {
        assert_eq!(
            build_cnb_url("2.0.2-beta.1"),
            "https://cnb.cool/SmailPang/SkiHide/-/releases/download/2.0.2-beta.1/SkiHide-2.0.2-beta.1.exe"
        );
    }

    #[test]
    fn cnb_source_is_preferred_before_github() {
        assert_eq!(
            build_download_candidates("cnb", "2.0.2-beta.1", None),
            vec![
                "https://cnb.cool/SmailPang/SkiHide/-/releases/download/2.0.2-beta.1/SkiHide-2.0.2-beta.1.exe",
                "https://github.com/SmailPang/SkiHide/releases/download/2.0.2-beta.1/SkiHide-2.0.2-beta.1.exe",
            ]
        );
    }

    #[test]
    fn cloudflare_source_is_preferred_with_existing_fallbacks() {
        assert_eq!(
            build_download_candidates(
                "cloudflare",
                "2.0.3",
                Some("https://example.workers.dev/api/download?asset_id=1"),
            ),
            vec![
                "https://example.workers.dev/api/download?asset_id=1",
                "https://cnb.cool/SmailPang/SkiHide/-/releases/download/2.0.3/SkiHide-2.0.3.exe",
                "https://github.com/SmailPang/SkiHide/releases/download/2.0.3/SkiHide-2.0.3.exe",
            ]
        );
    }

    #[test]
    fn update_user_agent_contains_software_name_and_version() {
        assert_eq!(
            update_user_agent(),
            format!("SkiHide/{}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn update_channels_map_to_cloudflare_api_values() {
        assert_eq!(cloudflare_channel("beta"), "prerelease");
        assert_eq!(cloudflare_channel("stable"), "release");
    }
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
