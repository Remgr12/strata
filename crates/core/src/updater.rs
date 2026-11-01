use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const DEFAULT_RELEASES_API_URL: &str =
    "https://git.soventa.tech/api/v1/repos/moha/strata/releases";
pub const DEFAULT_RELEASES_PAGE_URL: &str = "https://git.soventa.tech/moha/strata/releases";
pub const UPDATE_CHECK_INTERVAL_SECS: u64 = 900; // 15 minutes
pub const FLATPAK_APP_ID: &str = "tech.soventa.strata";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallKind {
    WindowsMsi,
    WindowsPortable,
    LinuxNative,
    LinuxFlatpak,
}

impl InstallKind {
    pub fn detect() -> Self {
        if Self::is_flatpak_runtime() {
            return InstallKind::LinuxFlatpak;
        }
        if cfg!(target_os = "windows") {
            if let Ok(exe) = std::env::current_exe() {
                let path_lower = exe.to_string_lossy().to_lowercase();
                if path_lower.contains("program files") || path_lower.contains("strata\\strata.exe")
                {
                    return InstallKind::WindowsMsi;
                }
            }
            InstallKind::WindowsMsi
        } else {
            InstallKind::LinuxNative
        }
    }

    pub fn is_flatpak_runtime() -> bool {
        std::env::var("FLATPAK_ID").is_ok() || Path::new("/.flatpak-info").exists()
    }

    pub fn label(&self) -> &'static str {
        match self {
            InstallKind::WindowsMsi => "Windows MSI",
            InstallKind::WindowsPortable => "Windows Portable",
            InstallKind::LinuxNative => "Linux Native",
            InstallKind::LinuxFlatpak => "Linux Flatpak",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub install_kind: InstallKind,
    pub release: Option<ReleaseInfo>,
    pub selected_asset: Option<ReleaseAsset>,
}

/// Configure a Command so that on Windows it never spawns a visible CMD console window.
fn configure_no_window(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

/// Parse a semantic/dotted version string (e.g. "v0.0.3.1" -> [0, 0, 3, 1]).
pub fn parse_version(version: &str) -> Vec<u32> {
    let clean = version
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or("");
    clean
        .split('.')
        .filter_map(|part| part.trim().parse::<u32>().ok())
        .collect()
}

/// Returns true if `candidate` is strictly newer than `current`.
pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    let mut cur = parse_version(current);
    let mut cand = parse_version(candidate);
    if cand.is_empty() {
        return false;
    }
    let max_len = cur.len().max(cand.len());
    cur.resize(max_len, 0);
    cand.resize(max_len, 0);
    cand > cur
}

/// Select the most appropriate release asset for the given `InstallKind`.
pub fn select_asset_for_install<'a>(
    release: &'a ReleaseInfo,
    kind: InstallKind,
) -> Option<&'a ReleaseAsset> {
    let preferred_names: &[&str] = match kind {
        InstallKind::WindowsMsi => &[
            "strata-windows-x86_64.msi",
            "strata.msi",
            "strata-windows-x86_64.exe",
            "strata.exe",
        ],
        InstallKind::WindowsPortable => &[
            "strata-windows-x86_64.exe",
            "strata.exe",
            "strata-windows-x86_64.msi",
        ],
        InstallKind::LinuxFlatpak => &[
            "strata-linux-x86_64.flatpak",
            "tech.soventa.strata.flatpak",
            "strata.flatpak",
            "strata-linux",
        ],
        InstallKind::LinuxNative => &["strata-linux", "strata-linux-x86_64.tar.gz"],
    };

    for pref in preferred_names {
        if let Some(asset) = release.assets.iter().find(|a| a.name == *pref) {
            return Some(asset);
        }
    }
    None
}

/// Parse a SHA256SUMS file into a map of `filename -> hex_digest`.
pub fn parse_sha256sums(contents: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        if let (Some(hash), Some(fname)) = (parts.next(), parts.next()) {
            let clean_name = fname.trim_start_matches('*').to_string();
            map.insert(clean_name, hash.to_lowercase());
        }
    }
    map
}

/// Pure-Rust FIPS 180-4 SHA-256 implementation for zero-dependency checksum verification.
pub fn sha256_bytes(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    format!(
        "{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
        h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]
    )
}

/// Compute SHA-256 hex digest of a file on disk.
pub fn compute_file_sha256(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open {} for hashing", path.display()))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(sha256_bytes(&bytes))
}

fn http_get_string(url: &str) -> Result<String> {
    let mut cmd = Command::new("curl");
    configure_no_window(&mut cmd);
    let output = cmd
        .args([
            "-s",
            "-S",
            "-L",
            "--fail",
            "--max-time",
            "15",
            "-H",
            "Accept: application/json",
            "-A",
            "Strata-Updater/1.0",
            url,
        ])
        .output()
        .with_context(|| format!("Failed to invoke curl for {}", url))?;

    if !output.status.success() {
        return Err(anyhow!(
            "HTTP request failed ({}): {}",
            url,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Fetch releases from the Forgejo API (with fallback to releases page tag parsing).
pub fn fetch_releases(api_url: &str) -> Result<Vec<ReleaseInfo>> {
    match http_get_string(api_url) {
        Ok(body_str) => {
            let releases: Vec<ReleaseInfo> =
                serde_json::from_str(&body_str).context("Failed to parse releases JSON")?;
            Ok(releases)
        }
        Err(api_err) => {
            let html = http_get_string(DEFAULT_RELEASES_PAGE_URL)
                .map_err(|_| anyhow!("Failed to query releases API: {}", api_err))?;
            let mut fallback_releases = Vec::new();
            for segment in html.split("/releases/tag/") {
                if let Some(tag) = segment.split(['"', '\'', '<', '>', ' ', '?']).next() {
                    if tag.starts_with('v') && !parse_version(tag).is_empty() {
                        let base_dl = format!("{}/download/{}", DEFAULT_RELEASES_PAGE_URL, tag);
                        fallback_releases.push(ReleaseInfo {
                            tag_name: tag.to_string(),
                            name: format!("Strata {}", tag),
                            body: String::new(),
                            html_url: format!("{}/tag/{}", DEFAULT_RELEASES_PAGE_URL, tag),
                            prerelease: false,
                            draft: false,
                            assets: vec![
                                ReleaseAsset {
                                    name: "strata-windows-x86_64.msi".into(),
                                    browser_download_url: format!(
                                        "{}/strata-windows-x86_64.msi",
                                        base_dl
                                    ),
                                    size: 0,
                                },
                                ReleaseAsset {
                                    name: "strata-windows-x86_64.exe".into(),
                                    browser_download_url: format!(
                                        "{}/strata-windows-x86_64.exe",
                                        base_dl
                                    ),
                                    size: 0,
                                },
                                ReleaseAsset {
                                    name: "strata-linux".into(),
                                    browser_download_url: format!("{}/strata-linux", base_dl),
                                    size: 0,
                                },
                                ReleaseAsset {
                                    name: "strata-linux-x86_64.flatpak".into(),
                                    browser_download_url: format!(
                                        "{}/strata-linux-x86_64.flatpak",
                                        base_dl
                                    ),
                                    size: 0,
                                },
                                ReleaseAsset {
                                    name: "SHA256SUMS".into(),
                                    browser_download_url: format!("{}/SHA256SUMS", base_dl),
                                    size: 0,
                                },
                            ],
                        });
                        break;
                    }
                }
            }
            if fallback_releases.is_empty() {
                Err(anyhow!("No releases found at {}", api_url))
            } else {
                Ok(fallback_releases)
            }
        }
    }
}

/// Check if a newer release is available compared to `current_version`.
pub fn check_for_updates(current_version: &str) -> Result<UpdateCheckResult> {
    check_for_updates_with_url(current_version, DEFAULT_RELEASES_API_URL)
}

pub fn check_for_updates_with_url(
    current_version: &str,
    api_url: &str,
) -> Result<UpdateCheckResult> {
    let install_kind = InstallKind::detect();
    let releases = fetch_releases(api_url)?;

    let mut best_release: Option<ReleaseInfo> = None;
    for rel in releases.into_iter().filter(|r| !r.draft) {
        match &best_release {
            None => best_release = Some(rel),
            Some(existing) => {
                if is_newer_version(&existing.tag_name, &rel.tag_name) {
                    best_release = Some(rel);
                }
            }
        }
    }

    if let Some(rel) = best_release {
        let latest_version = rel.tag_name.trim_start_matches('v').to_string();
        let update_available = is_newer_version(current_version, &rel.tag_name);
        let selected_asset = select_asset_for_install(&rel, install_kind).cloned();
        Ok(UpdateCheckResult {
            current_version: current_version.trim_start_matches('v').to_string(),
            latest_version,
            update_available,
            install_kind,
            release: Some(rel),
            selected_asset,
        })
    } else {
        Ok(UpdateCheckResult {
            current_version: current_version.trim_start_matches('v').to_string(),
            latest_version: current_version.trim_start_matches('v').to_string(),
            update_available: false,
            install_kind,
            release: None,
            selected_asset: None,
        })
    }
}

fn download_url_to_path(url: &str, dest: &Path) -> Result<()> {
    let mut cmd = Command::new("curl");
    configure_no_window(&mut cmd);
    let status = cmd
        .args(["-s", "-S", "-L", "--fail", "-A", "Strata-Updater/1.0", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .with_context(|| format!("Failed to invoke curl to download {}", url))?;

    if !status.success() {
        return Err(anyhow!("Failed to download release asset from {}", url));
    }
    Ok(())
}

/// Download and apply the update for the current platform and installation kind.
pub fn download_and_apply_update(check: &UpdateCheckResult) -> Result<String> {
    let release = check
        .release
        .as_ref()
        .ok_or_else(|| anyhow!("No release metadata available to download"))?;

    let asset = select_asset_for_install(release, check.install_kind).ok_or_else(|| {
        anyhow!(
            "No compatible release asset found for {}",
            check.install_kind.label()
        )
    })?;

    let temp_dir = std::env::temp_dir().join(format!("strata-update-{}", check.latest_version));
    std::fs::create_dir_all(&temp_dir)?;

    let downloaded_path = temp_dir.join(&asset.name);
    download_url_to_path(&asset.browser_download_url, &downloaded_path)?;

    if let Some(sums_asset) = release.assets.iter().find(|a| a.name == "SHA256SUMS") {
        let sums_path = temp_dir.join("SHA256SUMS");
        if download_url_to_path(&sums_asset.browser_download_url, &sums_path).is_ok() {
            if let Ok(sums_text) = std::fs::read_to_string(&sums_path) {
                let sums = parse_sha256sums(&sums_text);
                if let Some(expected_hash) = sums.get(&asset.name) {
                    let actual_hash = compute_file_sha256(&downloaded_path)?;
                    if &actual_hash != expected_hash {
                        let _ = std::fs::remove_file(&downloaded_path);
                        return Err(anyhow!(
                            "SHA-256 checksum mismatch for {} (expected {}, got {})",
                            asset.name,
                            expected_hash,
                            actual_hash
                        ));
                    }
                }
            }
        }
    }

    match check.install_kind {
        InstallKind::WindowsMsi => {
            apply_windows_msi_update(&downloaded_path, &check.latest_version)
        }
        InstallKind::WindowsPortable => {
            apply_windows_portable_update(&downloaded_path, &check.latest_version)
        }
        InstallKind::LinuxFlatpak => {
            apply_linux_flatpak_update(&downloaded_path, &check.latest_version)
        }
        InstallKind::LinuxNative => {
            apply_linux_native_update(&downloaded_path, &check.latest_version)
        }
    }
}

fn apply_windows_msi_update(downloaded_path: &Path, version: &str) -> Result<String> {
    if downloaded_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
    {
        return apply_windows_portable_update(downloaded_path, version);
    }

    let mut cmd = Command::new("msiexec");
    configure_no_window(&mut cmd);
    cmd.arg("/i")
        .arg(downloaded_path)
        .arg("/passive")
        .arg("/norestart")
        .spawn()
        .context("Failed to launch Windows MSI installer (msiexec)")?;

    Ok(format!(
        "Launched Windows installer for v{}. Strata will update automatically.",
        version
    ))
}

fn apply_windows_portable_update(downloaded_path: &Path, version: &str) -> Result<String> {
    let current_exe =
        std::env::current_exe().context("Could not determine current executable path")?;
    let backup_exe = current_exe.with_extension("exe.old");
    let _ = std::fs::remove_file(&backup_exe);
    std::fs::rename(&current_exe, &backup_exe)
        .context("Failed to rename active executable for update")?;
    if let Err(e) = std::fs::copy(downloaded_path, &current_exe) {
        let _ = std::fs::rename(&backup_exe, &current_exe);
        return Err(anyhow!("Failed to replace executable: {}", e));
    }
    Ok(format!(
        "Updated binary to v{}. Restart Strata to run the new version.",
        version
    ))
}

fn apply_linux_flatpak_update(downloaded_path: &Path, version: &str) -> Result<String> {
    let is_bundle = downloaded_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("flatpak"))
        .unwrap_or(false);

    if is_bundle {
        let status = if InstallKind::is_flatpak_runtime() {
            Command::new("flatpak-spawn")
                .args([
                    "--host",
                    "flatpak",
                    "install",
                    "--user",
                    "-y",
                    "--or-update",
                    "--bundle",
                ])
                .arg(downloaded_path)
                .status()
        } else {
            Command::new("flatpak")
                .args(["install", "--user", "-y", "--or-update", "--bundle"])
                .arg(downloaded_path)
                .status()
        };

        if let Ok(s) = status {
            if s.success() {
                return Ok(format!(
                    "Installed Flatpak update v{}. Restart Strata to apply.",
                    version
                ));
            }
        }
    }

    let host_update = Command::new("flatpak-spawn")
        .args(["--host", "flatpak", "update", "-y", FLATPAK_APP_ID])
        .status();

    match host_update {
        Ok(s) if s.success() => Ok(format!(
            "Flatpak updated to v{}. Restart Strata to complete.",
            version
        )),
        _ => Ok(format!(
            "Downloaded v{} bundle to {}. Run: flatpak install --user --bundle {}",
            version,
            downloaded_path.display(),
            downloaded_path.display()
        )),
    }
}

fn apply_linux_native_update(downloaded_path: &Path, version: &str) -> Result<String> {
    let binary_path: PathBuf = if downloaded_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(".tar.gz"))
        .unwrap_or(false)
    {
        let extract_dir = downloaded_path.parent().unwrap().join("extracted");
        std::fs::create_dir_all(&extract_dir)?;
        let _ = Command::new("tar")
            .arg("-xzf")
            .arg(downloaded_path)
            .arg("-C")
            .arg(&extract_dir)
            .status();
        let extracted_bin = extract_dir.join("strata-linux");
        if extracted_bin.exists() {
            extracted_bin
        } else {
            downloaded_path.to_path_buf()
        }
    } else {
        downloaded_path.to_path_buf()
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&binary_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&binary_path, perms)?;
    }

    if let Ok(current_exe) = std::env::current_exe() {
        let staging = current_exe.with_extension("new");
        if std::fs::copy(&binary_path, &staging).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&staging) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&staging, perms);
                }
            }
            if std::fs::rename(&staging, &current_exe).is_ok() {
                return Ok(format!(
                    "Updated Strata binary to v{}. Restart application to use the new release.",
                    version
                ));
            }
            let _ = std::fs::remove_file(&staging);
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let local_bin_dir = PathBuf::from(home).join(".local/bin");
        let _ = std::fs::create_dir_all(&local_bin_dir);
        let target = local_bin_dir.join("strata");
        std::fs::copy(&binary_path, &target)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&target)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&target, perms)?;
        }
        return Ok(format!(
            "Installed Strata v{} to {}. Restart to use the updated version.",
            version,
            target.display()
        ));
    }

    Ok(format!(
        "Downloaded Strata v{} to {}",
        version,
        binary_path.display()
    ))
}
