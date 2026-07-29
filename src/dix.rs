use std::path::Path;
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
    let generations_output = run_command(
        Command::new("home-manager")
            .arg("generations")
            .current_dir(flake_dir),
        &format!("home-manager generations in {}", flake_dir.display()),
    )?;
    let generations = String::from_utf8_lossy(&generations_output.stdout);
    let old_generation = parse_current_home_manager_generation(&generations)?;
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

fn parse_current_home_manager_generation(output: &str) -> Result<&str, String> {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .and_then(|line| line.split_once("->").map(|(_, path)| path.trim()))
        .and_then(|path| path.split_whitespace().next())
        .filter(|path| !path.is_empty())
        .ok_or_else(|| {
            "home-manager generations did not print a current generation path".to_owned()
        })
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
    fn parses_current_home_manager_generation_path() {
        let path = parse_current_home_manager_generation(
            r#"
2026-07-29 09:40 : id 925 -> /nix/store/d6mf0l1x3qq5sffzq0p2r68q3y5yvc9m-home-manager-generation (current)
2026-07-28 14:11 : id 924 -> /nix/store/460hk2172k6hv7v2m6rpjg481ncy49zn-home-manager-generation
"#,
        )
        .unwrap();

        assert_eq!(
            path,
            "/nix/store/d6mf0l1x3qq5sffzq0p2r68q3y5yvc9m-home-manager-generation"
        );
    }

    #[test]
    fn rejects_missing_home_manager_generation_path() {
        let err = parse_current_home_manager_generation("").unwrap_err();

        assert!(err.contains("did not print a current generation path"));
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
}
