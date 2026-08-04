use crate::format::{format_bytes, format_signed_bytes};
use crate::model::{PackageChange, ReportTotals, UpdateReport};

pub fn render_report(report: &UpdateReport, source_label: &str) -> String {
    render_report_with_title(
        report,
        &format!("{source_label} #{} :rocket:", report.flake),
    )
}

pub fn render_report_with_title(report: &UpdateReport, title: &str) -> String {
    let mut markdown = String::new();
    markdown.push_str(&format!("## SUNix Report: {}\n\n", escape_text(title)));
    markdown.push_str(&format!("**Summary:** {}\n", summary(report)));

    if let Some(metrics) = metrics(report) {
        markdown.push('\n');
        markdown.push_str(&metrics);
    }

    for group in report.groups.iter().filter(|group| !group.items.is_empty()) {
        markdown.push('\n');
        markdown.push_str("<details>\n");
        markdown.push_str(&format!(
            "<summary><strong>{}</strong> ({})</summary>\n\n",
            escape_html(group.status.heading()),
            group.items.len()
        ));

        if group.status.has_old_new_versions() {
            markdown.push_str("| Category | Name | Old Version | New Version | Size |\n");
            markdown.push_str("| --- | --- | --- | --- | --- |\n");
            for item in &group.items {
                markdown.push_str(&old_new_row(item, group.status.default_marker()));
            }
        } else {
            markdown.push_str("| Category | Name | Version | Size |\n");
            markdown.push_str("| --- | --- | --- | --- |\n");
            for item in &group.items {
                markdown.push_str(&version_row(item, group.status.default_marker()));
            }
        }

        markdown.push_str("\n</details>\n");
    }

    markdown
}

fn summary(report: &UpdateReport) -> String {
    let counts = report
        .groups
        .iter()
        .filter(|group| !group.items.is_empty())
        .map(|group| format!("{} {}", group.items.len(), group.status.summary_label()))
        .collect::<Vec<_>>();

    if counts.is_empty() {
        "No changes".to_owned()
    } else {
        escape_text(&counts.join(", "))
    }
}

fn metrics(report: &UpdateReport) -> Option<String> {
    let mut lines = Vec::new();

    if let Some(paths) = &report.totals.paths {
        lines.push(format!(
            "- **Paths:** {} -> {} (+{}, -{})",
            paths.old, paths.new, paths.added, paths.removed
        ));
    }

    if let Some(size) = size_metrics(&report.totals) {
        lines.push(size);
    }

    (!lines.is_empty()).then(|| format!("{}\n", lines.join("\n")))
}

fn size_metrics(totals: &ReportTotals) -> Option<String> {
    let (Some(size_old), Some(size_new)) = (totals.size_old, totals.size_new) else {
        return None;
    };

    Some(format!(
        "- **Size:** {} -> {} ({})",
        format_bytes(size_old),
        format_bytes(size_new),
        format_signed_bytes(size_new - size_old)
    ))
}

fn old_new_row(item: &PackageChange, marker: &str) -> String {
    format!(
        "| {} | {} | {} | {} | {} |\n",
        table_cell(marker),
        table_cell(&item.name),
        table_cell(item.old_version.as_deref().unwrap_or("-")),
        table_cell(item.new_version.as_deref().unwrap_or("-")),
        table_cell(&item.size),
    )
}

fn version_row(item: &PackageChange, marker: &str) -> String {
    format!(
        "| {} | {} | {} | {} |\n",
        table_cell(marker),
        table_cell(&item.name),
        table_cell(&item.version),
        table_cell(&item.size),
    )
}

fn table_cell(value: &str) -> String {
    escape_text(value).replace('\n', "<br>")
}

fn escape_text(value: &str) -> String {
    value.replace('\\', "\\\\").replace('|', "\\|")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use crate::model::{ChangeGroup, ChangeStatus, PackageChange, PathSummary, ReportTotals};

    use super::*;

    #[test]
    fn renders_report_as_github_markdown() {
        let report = sample_report();

        let markdown = render_report(&report, "NixOS");

        assert!(markdown.starts_with("## SUNix Report: NixOS #aorus :rocket:\n\n"));
        assert!(markdown.contains("**Summary:** 1 added, 1 upgraded"));
        assert!(markdown.contains("- **Paths:** 10 -> 12 (+3, -1)"));
        assert!(markdown.contains("- **Size:** 2 KiB -> 3 KiB (+1 KiB)"));
        let visible = markdown
            .split("<details>")
            .next()
            .expect("report should have visible content before details");
        assert!(visible.contains("## SUNix Report: NixOS #aorus :rocket:"));
        assert!(visible.contains("**Summary:** 1 added, 1 upgraded"));
        assert!(visible.contains("- **Paths:** 10 -> 12 (+3, -1)"));
        assert!(visible.contains("- **Size:** 2 KiB -> 3 KiB (+1 KiB)"));
        assert!(markdown.contains("<summary><strong>Added</strong> (1)</summary>"));
        assert!(markdown.contains("<summary><strong>Upgraded</strong> (1)</summary>"));
        assert!(markdown.contains("| Category | Name | Version | Size |"));
        assert!(markdown.contains("| [A+] | ripgrep | 14.1.1 | +5.1 MiB |"));
        assert!(markdown.contains("| Category | Name | Old Version | New Version | Size |"));
        assert!(markdown.contains("| [U.] | linux | 6.12.1 | 6.12.2 | +14 MiB |"));
        assert_eq!(markdown.matches("<details>").count(), 2);
        assert_eq!(markdown.matches("</details>").count(), 2);
    }

    #[test]
    fn renders_empty_reports_without_tables() {
        let report = UpdateReport {
            flake: "empty".to_owned(),
            groups: Vec::new(),
            totals: ReportTotals::default(),
        };

        let markdown = render_report(&report, "Demo");

        assert!(markdown.contains("**Summary:** No changes"));
        assert!(!markdown.contains("| Category |"));
    }

    #[test]
    fn renders_report_with_cli_title() {
        let report = sample_report();

        let markdown = render_report_with_title(&report, "NixOS .#aorus");

        assert!(markdown.starts_with("## SUNix Report: NixOS .#aorus\n\n"));
        assert!(!markdown.contains(":rocket:"));
    }

    #[test]
    fn escapes_table_cells() {
        let report = UpdateReport {
            flake: "flake`name".to_owned(),
            groups: vec![ChangeGroup {
                status: ChangeStatus::Changed,
                items: vec![PackageChange {
                    name: "pkg|name".to_owned(),
                    version: "1\\2\n3".to_owned(),
                    old_version: None,
                    new_version: None,
                    size: "+1 B".to_owned(),
                }],
            }],
            totals: ReportTotals::default(),
        };

        let markdown = render_report(&report, "Demo");

        assert!(markdown.contains("## SUNix Report: Demo #flake`name :rocket:"));
        assert!(markdown.contains("| [C] | pkg\\|name | 1\\\\2<br>3 | +1 B |"));
    }

    fn sample_report() -> UpdateReport {
        UpdateReport {
            flake: "aorus".to_owned(),
            groups: vec![
                ChangeGroup {
                    status: ChangeStatus::Added,
                    items: vec![PackageChange {
                        name: "ripgrep".to_owned(),
                        version: "14.1.1".to_owned(),
                        old_version: None,
                        new_version: None,
                        size: "+5.1 MiB".to_owned(),
                    }],
                },
                ChangeGroup {
                    status: ChangeStatus::Upgraded,
                    items: vec![PackageChange {
                        name: "linux".to_owned(),
                        version: "6.12.1 -> 6.12.2".to_owned(),
                        old_version: Some("6.12.1".to_owned()),
                        new_version: Some("6.12.2".to_owned()),
                        size: "+14 MiB".to_owned(),
                    }],
                },
            ],
            totals: ReportTotals {
                paths: Some(PathSummary {
                    old: 10,
                    new: 12,
                    added: 3,
                    removed: 1,
                }),
                size_old: Some(2048),
                size_new: Some(3072),
            },
        }
    }
}
