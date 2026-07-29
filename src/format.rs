use std::cmp::Ordering;

use crate::dix::{DixVersion, DixVersionDiff};
use crate::model::{ChangeStatus, VersionChange};

const SINGLE_VERSION_OUTPUT_SUFFIXES: &[&str] = &[
    "-fish-completions",
    "-fhsenv-profile",
    "-fhsenv-rootfs",
    "-libgcc",
    "-bwrap",
    "-extracted",
    "-patched",
    "-init",
    "-bin",
    "-dev",
    "-doc",
    "-info",
    "-lib",
    "-man",
    "-out",
    "-static",
    "-debug",
];

pub fn format_versions(versions: &[DixVersionDiff], has_omitted_versions: bool) -> String {
    let mut names = versions.iter().map(format_version).collect::<Vec<_>>();

    if has_omitted_versions {
        names.push("...".to_owned());
    }

    if names.is_empty() {
        "-".to_owned()
    } else {
        names.join(", ")
    }
}

pub fn format_version_change(
    status: &ChangeStatus,
    versions: &[DixVersionDiff],
    has_omitted_versions: bool,
) -> VersionChange {
    if let Some((old, new)) = versions
        .iter()
        .filter_map(compact_changed_version_pair)
        .min_by_key(|(old, new)| old.len() + new.len())
    {
        return orient_version_change(status, old, new);
    }

    if matches!(status, ChangeStatus::Downgraded)
        && let Some(version_change) = compact_downgrade_versions(versions)
    {
        return version_change;
    }

    let mut old = compact_version_with_kind(versions, "removed");
    let mut new = compact_version_with_kind(versions, "added");

    if has_omitted_versions {
        match (&old, &new) {
            (None, Some(_)) => old = Some("...".to_owned()),
            (Some(_), None) => new = Some("...".to_owned()),
            (None, None) => old = Some("...".to_owned()),
            (Some(_), Some(_)) => {}
        }
    }

    VersionChange { old, new }
}

fn orient_version_change(status: &ChangeStatus, old: String, new: String) -> VersionChange {
    if matches!(status, ChangeStatus::Downgraded) && compare_versions(&old, &new).is_lt() {
        return VersionChange {
            old: Some(new),
            new: Some(old),
        };
    }

    VersionChange {
        old: Some(old),
        new: Some(new),
    }
}

fn compact_downgrade_versions(versions: &[DixVersionDiff]) -> Option<VersionChange> {
    let mut candidates = versions
        .iter()
        .filter_map(|version| {
            if matches!(
                version.kind.as_str(),
                "added" | "removed" | "amount_changed"
            ) {
                version
                    .version
                    .as_ref()
                    .map(|version| compact_single_version(&version.name))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| compare_versions(left, right));
    candidates.dedup();

    let new = candidates.first()?.clone();
    let old = candidates.last()?.clone();

    (old != new).then_some(VersionChange {
        old: Some(old),
        new: Some(new),
    })
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let left_parts = numeric_version_parts(left);
    let right_parts = numeric_version_parts(right);
    let max_len = left_parts.len().max(right_parts.len());

    for index in 0..max_len {
        let left_part = left_parts.get(index).copied().unwrap_or_default();
        let right_part = right_parts.get(index).copied().unwrap_or_default();

        match left_part.cmp(&right_part) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }

    left.cmp(right)
}

fn numeric_version_parts(version: &str) -> Vec<u64> {
    let mut parts = Vec::new();
    let mut current = String::new();

    for char in version.chars() {
        if char.is_ascii_digit() {
            current.push(char);
        } else if !current.is_empty() {
            parts.push(current.parse().unwrap_or(u64::MAX));
            current.clear();
        }
    }

    if !current.is_empty() {
        parts.push(current.parse().unwrap_or(u64::MAX));
    }

    parts
}

pub fn format_signed_bytes(bytes: i64) -> String {
    match bytes.cmp(&0) {
        std::cmp::Ordering::Greater => format!("+{}", format_bytes(bytes)),
        std::cmp::Ordering::Less => format!("-{}", format_bytes(bytes.abs())),
        std::cmp::Ordering::Equal => format_bytes(0),
    }
}

pub fn format_bytes(bytes: i64) -> String {
    let mut value = bytes.abs() as f64;
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut unit = units[0];

    for next_unit in &units[1..] {
        if value < 1024.0 {
            break;
        }

        value /= 1024.0;
        unit = next_unit;
    }

    if unit == "B" {
        return format!("{} B", bytes.abs());
    }

    let precision = if value >= 100.0 {
        0
    } else if value >= 10.0 {
        1
    } else {
        2
    };

    format!(
        "{} {}",
        trim_float(format!("{:.*}", precision, value)),
        unit
    )
}

fn compact_changed_version_pair(version: &DixVersionDiff) -> Option<(String, String)> {
    if version.kind != "changed" {
        return None;
    }

    let old = version.old.as_ref()?;
    let new = version.new.as_ref()?;

    Some(compact_pair(&old.name, &new.name))
}

fn compact_pair(old: &str, new: &str) -> (String, String) {
    strip_common_output_suffix(old, new).unwrap_or_else(|| (old.to_owned(), new.to_owned()))
}

fn strip_common_output_suffix(old: &str, new: &str) -> Option<(String, String)> {
    let old_parts = old.split('-').collect::<Vec<_>>();
    let new_parts = new.split('-').collect::<Vec<_>>();

    if old_parts.len() < 2 || new_parts.len() < 2 {
        return None;
    }

    let max_suffix_len = old_parts.len().min(new_parts.len()) - 1;
    let mut suffix_len = 0;

    while suffix_len < max_suffix_len {
        let old_part = old_parts[old_parts.len() - 1 - suffix_len];
        let new_part = new_parts[new_parts.len() - 1 - suffix_len];

        if old_part != new_part || !looks_like_output_suffix_part(old_part) {
            break;
        }

        suffix_len += 1;
    }

    if suffix_len == 0 {
        return None;
    }

    let old_base = old_parts[..old_parts.len() - suffix_len].join("-");
    let new_base = new_parts[..new_parts.len() - suffix_len].join("-");

    if old_base.is_empty()
        || new_base.is_empty()
        || !old_base.chars().any(|char| char.is_ascii_digit())
        || !new_base.chars().any(|char| char.is_ascii_digit())
    {
        return None;
    }

    Some((old_base, new_base))
}

fn looks_like_output_suffix_part(part: &str) -> bool {
    part.chars().any(|char| char.is_ascii_alphabetic())
        && part
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '_' | '.'))
}

fn compact_version_with_kind(versions: &[DixVersionDiff], kind: &str) -> Option<String> {
    versions
        .iter()
        .filter(|version| version.kind == kind)
        .filter_map(|version| version.version.as_ref())
        .map(|version| compact_single_version(&version.name))
        .min_by_key(|version| version.len())
}

fn compact_single_version(version: &str) -> String {
    SINGLE_VERSION_OUTPUT_SUFFIXES
        .iter()
        .find_map(|suffix| {
            version
                .strip_suffix(suffix)
                .filter(|base| base.chars().any(|char| char.is_ascii_digit()))
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| version.to_owned())
}

fn format_version(version: &DixVersionDiff) -> String {
    match version.kind.as_str() {
        "amount_changed" => {
            if let (Some(package_version), Some(old_amount), Some(new_amount)) =
                (&version.version, version.old_amount, version.new_amount)
            {
                format!(
                    "{} ({} -> {})",
                    package_version.name, old_amount, new_amount
                )
            } else {
                format_missing_version(version)
            }
        }
        "changed" => {
            if let (Some(old), Some(new)) = (&version.old, &version.new) {
                format!(
                    "{} -> {}",
                    format_named_version(old),
                    format_named_version(new)
                )
            } else {
                format_missing_version(version)
            }
        }
        "added" | "removed" => version
            .version
            .as_ref()
            .map(format_named_version)
            .unwrap_or_else(|| format_missing_version(version)),
        _ => version
            .version
            .as_ref()
            .map(format_named_version)
            .or_else(|| {
                version
                    .old
                    .as_ref()
                    .zip(version.new.as_ref())
                    .map(|(old, new)| {
                        format!(
                            "{} -> {}",
                            format_named_version(old),
                            format_named_version(new)
                        )
                    })
            })
            .unwrap_or_else(|| format_missing_version(version)),
    }
}

fn format_named_version(version: &DixVersion) -> String {
    match version.amount {
        Some(amount) if amount > 1 => format!("{} x{}", version.name, amount),
        _ => version.name.clone(),
    }
}

fn format_missing_version(version: &DixVersionDiff) -> String {
    format!("<{}>", version.kind)
}

fn trim_float(mut value: String) -> String {
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }

    if value.ends_with('.') {
        value.pop();
    }

    value
}
