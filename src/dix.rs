use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

use serde::Deserialize;

use crate::command::{run_command, run_command_with_logs};
use crate::config::SunixConfig;
use crate::format::{format_signed_bytes, format_version_change, format_versions};
use crate::model::{
    ChangeGroup, ChangeStatus, PackageChange, PathSummary, ReportTotals, UpdateReport,
};

const DEMO_FLAKE: &str = "demo";

pub fn demo_report() -> UpdateReport {
    parse_report_json(
        include_str!("../assets/sample.json"),
        "sample.json",
        DEMO_FLAKE,
    )
    .expect("embedded sample.json must parse")
}

pub fn home_manager_report_with_logs(
    config: &SunixConfig,
    logs: mpsc::Sender<String>,
) -> Result<UpdateReport, String> {
    home_manager_report(config, Some(&logs))
}

fn home_manager_report(
    config: &SunixConfig,
    logs: Option<&mpsc::Sender<String>>,
) -> Result<UpdateReport, String> {
    let flake_dir = config.home_flake_dir();
    let old_generation_path = current_home_manager_generation_path()?;
    let old_generation = old_generation_path.to_str().ok_or_else(|| {
        format!(
            "active Home Manager generation path {} is not valid UTF-8",
            old_generation_path.display()
        )
    })?;
    let attr = format!(
        ".#homeConfigurations.{}.activationPackage",
        config.home_flake
    );

    build_diff_report(
        config,
        flake_dir,
        &attr,
        old_generation,
        &config.home_flake,
        logs,
    )
}

pub fn nixos_report_with_logs(
    config: &SunixConfig,
    logs: mpsc::Sender<String>,
) -> Result<UpdateReport, String> {
    nixos_report(config, Some(&logs))
}

fn nixos_report(
    config: &SunixConfig,
    logs: Option<&mpsc::Sender<String>>,
) -> Result<UpdateReport, String> {
    let flake_dir = config.nixos_flake_dir();
    let attr = format!(
        ".#nixosConfigurations.{}.config.system.build.toplevel",
        config.nixos_flake
    );

    build_diff_report(
        config,
        flake_dir,
        &attr,
        "/run/current-system/",
        &config.nixos_flake,
        logs,
    )
}

fn build_diff_report(
    config: &SunixConfig,
    flake_dir: &Path,
    attr: &str,
    old_path: &str,
    flake: &str,
    logs: Option<&mpsc::Sender<String>>,
) -> Result<UpdateReport, String> {
    let output_paths = nix_build_output_paths(attr, flake_dir, logs)?;

    let dix_binary = dix_binary(config);
    let dix_output = run_command(
        Command::new(dix_binary)
            .arg("--output=json")
            .arg(old_path)
            .args(output_paths)
            .current_dir(flake_dir),
        &format!(
            "{} --output=json {old_path} in {}",
            dix_binary.display(),
            flake_dir.display()
        ),
    )?;
    let json = String::from_utf8_lossy(&dix_output.stdout);
    parse_report_json(&json, &format!("dix output for `{flake}`"), flake)
}

fn nix_build_output_paths(
    attr: &str,
    flake_dir: &Path,
    logs: Option<&mpsc::Sender<String>>,
) -> Result<Vec<String>, String> {
    let mut command = Command::new("nix");
    command
        .arg("build")
        .arg("--print-out-paths")
        .arg("--no-link")
        .arg(attr);

    let output = run_command_with_logs(
        command.current_dir(flake_dir),
        &format!("nix build {attr} in {}", flake_dir.display()),
        logs,
    )?;
    let output_paths = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if output_paths.is_empty() {
        return Err(format!(
            "nix build {attr} in {} did not print an output path",
            flake_dir.display()
        ));
    }

    Ok(output_paths)
}

fn current_home_manager_generation_path() -> Result<PathBuf, String> {
    resolve_current_home_manager_generation_path(&home_manager_profile_candidates()?)
}

fn home_manager_profile_candidates() -> Result<Vec<PathBuf>, String> {
    let mut candidates = Vec::new();

    if let Some(state_home) = env::var_os("XDG_STATE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".local/state"))
        })
    {
        candidates.push(state_home.join("nix/profiles/home-manager"));
    }

    if let Some(user) = env::var_os("USER")
        .filter(|value| !value.is_empty())
        .or_else(|| env::var_os("LOGNAME").filter(|value| !value.is_empty()))
    {
        candidates.push(
            PathBuf::from("/nix/var/nix/profiles/per-user")
                .join(user)
                .join("home-manager"),
        );
    }

    if candidates.is_empty() {
        return Err(
            "XDG_STATE_HOME, HOME, USER, and LOGNAME are unset; cannot locate Home Manager profile"
                .to_owned(),
        );
    }

    Ok(candidates)
}

fn resolve_current_home_manager_generation_path(candidates: &[PathBuf]) -> Result<PathBuf, String> {
    for candidate in candidates {
        match fs::canonicalize(candidate) {
            Ok(path) => return Ok(path),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(format!(
                    "failed to resolve Home Manager profile {}: {err}",
                    candidate.display()
                ));
            }
        }
    }

    let tried = candidates
        .iter()
        .map(|candidate| candidate.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "could not find active Home Manager profile; tried {tried}"
    ))
}

fn dix_binary(config: &SunixConfig) -> &Path {
    config
        .dix_binary
        .as_deref()
        .unwrap_or_else(|| Path::new("dix"))
}

pub fn parse_report_json(content: &str, source: &str, flake: &str) -> Result<UpdateReport, String> {
    serde_json::from_str::<DixReport>(content)
        .map(|report| report.into_update_report(flake))
        .map_err(|err| format!("failed to parse {source}: {err}"))
}

#[derive(Debug, Deserialize)]
struct DixReport {
    diffs: Vec<DixDiff>,
    #[serde(default)]
    paths: Option<DixPaths>,
    #[serde(default)]
    size_old: Option<i64>,
    #[serde(default)]
    size_new: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct DixPaths {
    old: i64,
    new: i64,
    added: i64,
    removed: i64,
}

#[derive(Debug, Deserialize)]
struct DixDiff {
    name: String,
    #[serde(default)]
    versions: Vec<DixVersionDiff>,
    status: String,
    #[serde(default)]
    has_omitted_versions: bool,
    #[serde(default)]
    size_delta: i64,
}

#[derive(Debug, Deserialize)]
pub struct DixVersionDiff {
    pub kind: String,
    #[serde(default)]
    pub version: Option<DixVersion>,
    #[serde(default)]
    pub old: Option<DixVersion>,
    #[serde(default)]
    pub new: Option<DixVersion>,
    #[serde(default)]
    pub old_amount: Option<i64>,
    #[serde(default)]
    pub new_amount: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DixVersion {
    pub name: String,
    #[serde(default)]
    pub amount: Option<i64>,
}

impl DixReport {
    fn into_update_report(self, flake: &str) -> UpdateReport {
        let mut groups: Vec<ChangeGroup> = Vec::new();

        for diff in self.diffs {
            let status = ChangeStatus::from_dix(diff.status.clone());
            let item = diff.into_package_change(&status);

            if let Some(group) = groups.iter_mut().find(|group| group.status == status) {
                group.items.push(item);
            } else {
                groups.push(ChangeGroup {
                    status,
                    items: vec![item],
                });
            }
        }

        groups.sort_by_key(|group| group.status.order());

        UpdateReport {
            flake: flake.to_owned(),
            groups,
            totals: ReportTotals {
                paths: self.paths.map(Into::into),
                size_old: self.size_old,
                size_new: self.size_new,
            },
        }
    }
}

impl From<DixPaths> for PathSummary {
    fn from(paths: DixPaths) -> Self {
        Self {
            old: paths.old,
            new: paths.new,
            added: paths.added,
            removed: paths.removed,
        }
    }
}

impl DixDiff {
    fn into_package_change(self, status: &ChangeStatus) -> PackageChange {
        let version_change = if status.has_old_new_versions() {
            format_version_change(status, &self.versions, self.has_omitted_versions)
        } else {
            Default::default()
        };

        PackageChange {
            name: self.name,
            version: format_versions(&self.versions, self.has_omitted_versions),
            old_version: version_change.old,
            new_version: version_change.new,
            size: format_signed_bytes(self.size_delta),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dix_sample_report() {
        let report = parse_report_json(
            include_str!("../assets/sample.json"),
            "sample.json",
            "aorus",
        )
        .unwrap();

        assert_eq!(report.flake, "aorus");
        assert!(report.totals.paths.is_some());
        assert!(report.totals.size_old.is_some());
        assert!(report.totals.size_new.is_some());
        assert!(!report.groups.is_empty());
        assert!(report.groups.iter().all(|group| !group.items.is_empty()));
    }

    #[test]
    fn builds_demo_report_from_sample_json() {
        let report = demo_report();

        assert_eq!(report.flake, "demo");
        assert!(report.totals.paths.is_some());
        assert!(!report.groups.is_empty());
    }

    #[test]
    fn resolves_current_home_manager_generation_profile_candidate() {
        let root = test_dir("hm-profile");
        let missing = root.join("missing-home-manager");
        let generation = root.join("generation");
        let profile = root.join("home-manager");
        std::fs::create_dir_all(&generation).unwrap();
        std::os::unix::fs::symlink(&generation, &profile).unwrap();

        let resolved = resolve_current_home_manager_generation_path(&[missing, profile]).unwrap();

        assert_eq!(resolved, std::fs::canonicalize(generation).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reports_missing_home_manager_profile_candidates() {
        let err = resolve_current_home_manager_generation_path(&[
            PathBuf::from("/missing-home-manager-profile-one"),
            PathBuf::from("/missing-home-manager-profile-two"),
        ])
        .unwrap_err();

        assert!(err.contains("could not find active Home Manager profile"));
        assert!(err.contains("/missing-home-manager-profile-one"));
        assert!(err.contains("/missing-home-manager-profile-two"));
    }

    #[test]
    fn parses_all_known_dix_version_entry_shapes() {
        let report = parse_report_json(
            r#"{
              "diffs": [
                {
                  "name": "bazecor",
                  "versions": [
                    {
                      "kind": "changed",
                      "old": { "name": "1.6.5-bwrap", "amount": 1 },
                      "new": { "name": "1.8.3-bwrap", "amount": 1 }
                    },
                    {
                      "kind": "added",
                      "version": { "name": "1.8.3-patched", "amount": 1 }
                    }
                  ],
                  "status": "Upgraded",
                  "has_omitted_versions": false,
                  "size_delta": 20408
                },
                {
                  "name": "gcc",
                  "versions": [
                    {
                      "kind": "changed",
                      "old": { "name": "14.2.1.20250322-lib", "amount": 1 },
                      "new": { "name": "14.3.0-lib", "amount": 1 }
                    },
                    {
                      "kind": "changed",
                      "old": { "name": "14.2.1.20250322-libgcc", "amount": 1 },
                      "new": { "name": "14.3.0-libgcc", "amount": 1 }
                    }
                  ],
                  "status": "Upgraded",
                  "has_omitted_versions": true,
                  "size_delta": -10519768
                },
                {
                  "name": "curl",
                  "versions": [
                    {
                      "kind": "removed",
                      "version": { "name": "8.13.0", "amount": 1 }
                    }
                  ],
                  "status": "Downgraded",
                  "has_omitted_versions": true,
                  "size_delta": -1099464
                },
                {
                  "name": "libffi",
                  "versions": [
                    {
                      "kind": "removed",
                      "version": { "name": "3.4.8", "amount": 1 }
                    },
                    {
                      "kind": "amount_changed",
                      "version": { "name": "3.5.2" },
                      "old_amount": 4,
                      "new_amount": 3
                    }
                  ],
                  "status": "Downgraded",
                  "has_omitted_versions": true,
                  "size_delta": -147424
                },
                {
                  "name": "libxml2",
                  "versions": [
                    {
                      "kind": "removed",
                      "version": { "name": "2.15.2", "amount": 1 }
                    },
                    {
                      "kind": "amount_changed",
                      "version": { "name": "2.15.1" },
                      "old_amount": 1,
                      "new_amount": 2
                    }
                  ],
                  "status": "Downgraded",
                  "has_omitted_versions": true,
                  "size_delta": -8400
                },
                {
                  "name": "ada",
                  "versions": [
                    {
                      "kind": "amount_changed",
                      "version": { "name": "3.4.4" },
                      "old_amount": 2,
                      "new_amount": 1
                    }
                  ],
                  "status": "Changed",
                  "has_omitted_versions": false,
                  "size_delta": -1368176
                },
                {
                  "name": "60-dygma.rules",
                  "versions": [
                    {
                      "kind": "added",
                      "version": { "name": "<none>", "amount": 1 }
                    }
                  ],
                  "status": "Added",
                  "has_omitted_versions": false,
                  "size_delta": 216
                }
              ],
              "paths": { "old": 1, "new": 2, "added": 1, "removed": 0 },
              "size_old": 1000,
              "size_new": 2000
            }"#,
            "inline",
            "aorus",
        )
        .unwrap();

        let upgraded = group(&report, &ChangeStatus::Upgraded);
        let downgraded = group(&report, &ChangeStatus::Downgraded);
        let changed = group(&report, &ChangeStatus::Changed);
        let added = group(&report, &ChangeStatus::Added);

        let bazecor = item(upgraded, "bazecor");
        assert_eq!(bazecor.version, "1.6.5-bwrap -> 1.8.3-bwrap, 1.8.3-patched");
        assert_eq!(bazecor.old_version.as_deref(), Some("1.6.5"));
        assert_eq!(bazecor.new_version.as_deref(), Some("1.8.3"));
        assert_eq!(bazecor.size, "+19.9 KiB");

        let gcc = item(upgraded, "gcc");
        assert_eq!(gcc.old_version.as_deref(), Some("14.2.1.20250322"));
        assert_eq!(gcc.new_version.as_deref(), Some("14.3.0"));
        assert_eq!(gcc.size, "-10 MiB");

        let curl = item(downgraded, "curl");
        assert_eq!(curl.old_version.as_deref(), Some("8.13.0"));
        assert_eq!(curl.new_version.as_deref(), Some("..."));
        assert_eq!(curl.size, "-1.05 MiB");

        let libffi = item(downgraded, "libffi");
        assert_eq!(libffi.old_version.as_deref(), Some("3.5.2"));
        assert_eq!(libffi.new_version.as_deref(), Some("3.4.8"));
        assert_eq!(libffi.size, "-144 KiB");

        let libxml2 = item(downgraded, "libxml2");
        assert_eq!(libxml2.old_version.as_deref(), Some("2.15.2"));
        assert_eq!(libxml2.new_version.as_deref(), Some("2.15.1"));
        assert_eq!(libxml2.size, "-8.2 KiB");

        let ada = item(changed, "ada");
        assert_eq!(ada.version, "3.4.4 (2 -> 1)");
        assert_eq!(ada.old_version, None);
        assert_eq!(ada.new_version, None);
        assert_eq!(ada.size, "-1.3 MiB");

        let dygma_rules = item(added, "60-dygma.rules");
        assert_eq!(dygma_rules.version, "<none>");
        assert_eq!(dygma_rules.size, "+216 B");
    }

    fn group<'a>(report: &'a UpdateReport, status: &ChangeStatus) -> &'a ChangeGroup {
        report
            .groups
            .iter()
            .find(|group| group.status == *status)
            .unwrap()
    }

    fn item<'a>(group: &'a ChangeGroup, name: &str) -> &'a PackageChange {
        group.items.iter().find(|item| item.name == name).unwrap()
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("sunix-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
