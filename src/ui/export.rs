use std::fs;
use std::path::{Path, PathBuf};

use gtk::gdk;
use gtk::prelude::*;

use crate::model::UpdateReport;
use crate::report_markdown;
use crate::report_pdf::save_report as save_pdf_report;

use super::state::ReportSource;
use super::widgets::{export_key_hints, header_icon_button};

pub(super) struct ExportControls {
    pub(super) button: gtk::Button,
    pub(super) popover: gtk::Popover,
}

pub(super) fn export_controls(source: ReportSource, report: &UpdateReport) -> ExportControls {
    let status = gtk::Label::new(None);
    status.add_css_class("export-status");
    status.set_halign(gtk::Align::Start);
    status.set_ellipsize(gtk::pango::EllipsizeMode::End);
    status.set_max_width_chars(42);

    let export = header_icon_button("document-save-symbolic", "Export report");

    let popover = gtk::Popover::new();
    popover.set_autohide(true);
    popover.set_has_arrow(false);
    let options = gtk::Box::new(gtk::Orientation::Vertical, 4);
    options.add_css_class("export-menu");

    let path = gtk::Entry::new();
    path.add_css_class("export-path");
    path.set_hexpand(true);
    path.set_width_chars(42);
    path.set_max_width_chars(64);
    path.set_activates_default(true);
    path.set_can_focus(true);
    path.set_focusable(true);
    path.set_text(&default_export_path(&report.flake, ExportFormat::Markdown));
    path.set_tooltip_text(Some("Export path"));
    options.append(&path);

    let formats = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    formats.add_css_class("export-formats");
    formats.set_halign(gtk::Align::Center);
    let pdf = export_option("PDF", ExportFormat::Pdf, &status, &path, source, report);
    let markdown = export_option(
        "Markdown",
        ExportFormat::Markdown,
        &status,
        &path,
        source,
        report,
    );
    formats.append(&pdf);
    formats.append(&markdown);
    options.append(&formats);
    options.append(&export_key_hints());
    options.append(&status);
    let default_export = markdown.clone();
    path.connect_activate(move |_| {
        default_export.emit_clicked();
    });

    popover.set_child(Some(&options));
    popover.set_default_widget(Some(&markdown));
    install_export_keybindings(&popover, &path, &pdf, &markdown);

    popover.connect_closed(cleanup_popover);

    let destroy_popover = popover.clone();
    export.connect_unrealize(move |_| cleanup_popover(&destroy_popover));

    let export_popover = popover.clone();
    let export_anchor = export.clone();
    let export_focus = markdown.clone();
    export.connect_clicked(move |_| {
        if export_popover.parent().is_none() {
            export_popover.set_parent(&export_anchor);
        }
        export_popover.popup();
        export_focus.grab_focus();

        let focus = export_focus.clone();
        gtk::glib::idle_add_local_once(move || {
            focus.grab_focus();
        });
    });

    ExportControls {
        button: export,
        popover,
    }
}

fn cleanup_popover(popover: &gtk::Popover) {
    if popover.parent().is_some() {
        popover.unparent();
    }
}

fn install_export_keybindings(
    popover: &gtk::Popover,
    path: &gtk::Entry,
    pdf: &gtk::Button,
    markdown: &gtk::Button,
) {
    install_export_key_handler(popover, popover, pdf, markdown);
    install_export_key_handler(path, popover, pdf, markdown);
    install_export_key_handler(pdf, popover, pdf, markdown);
    install_export_key_handler(markdown, popover, pdf, markdown);
}

fn install_export_key_handler<W>(
    widget: &W,
    popover: &gtk::Popover,
    pdf: &gtk::Button,
    markdown: &gtk::Button,
) where
    W: IsA<gtk::Widget>,
{
    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    let key_popover = popover.clone();
    let pdf = pdf.clone();
    let markdown = markdown.clone();
    key_controller.connect_key_pressed(move |_, keyval, _, modifiers| {
        if keyval == gdk::Key::Escape {
            key_popover.popdown();
            return gtk::glib::Propagation::Stop;
        }

        if !modifiers.contains(gdk::ModifierType::ALT_MASK) {
            return gtk::glib::Propagation::Proceed;
        }

        if key_matches(keyval, 'p') {
            pdf.emit_clicked();
            return gtk::glib::Propagation::Stop;
        }

        if key_matches(keyval, 'm') {
            markdown.emit_clicked();
            return gtk::glib::Propagation::Stop;
        }

        gtk::glib::Propagation::Proceed
    });

    widget.add_controller(key_controller);
}

fn key_matches(keyval: gdk::Key, expected: char) -> bool {
    keyval
        .to_unicode()
        .map(|value| value.eq_ignore_ascii_case(&expected))
        .unwrap_or(false)
}

fn export_option(
    label: &str,
    format: ExportFormat,
    status: &gtk::Label,
    path: &gtk::Entry,
    source: ReportSource,
    report: &UpdateReport,
) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("export-option");
    button.set_can_focus(true);
    button.set_focusable(true);
    button.set_focus_on_click(false);

    let status = status.clone();
    let path_entry = path.clone();
    let report = report.clone();
    button.connect_clicked(move |_| {
        match export_path_from_entry(path_entry.text().as_str(), &report.flake, format) {
            Ok(path) => {
                set_status(
                    &status,
                    &format!("Saving {}", format.label()),
                    StatusTone::Neutral,
                );
                match save_report(&report, source, format, &path) {
                    Ok(()) => {
                        path_entry.set_text(&path.display().to_string());
                        set_status(
                            &status,
                            &format!("Saved {}", short_path(&path)),
                            StatusTone::Success,
                        );
                    }
                    Err(message) => set_status(&status, &message, StatusTone::Error),
                }
            }
            Err(message) => set_status(&status, &message, StatusTone::Error),
        }
    });

    button
}

fn save_report(
    report: &UpdateReport,
    source: ReportSource,
    format: ExportFormat,
    path: &Path,
) -> Result<(), String> {
    match format {
        ExportFormat::Pdf => {
            let subtitle = export_subtitle(source, &report.flake);
            save_pdf_report(report, &subtitle, path)
        }
        ExportFormat::Markdown => {
            let markdown = report_markdown::render_report(report, source.label());
            fs::write(path, markdown)
                .map_err(|err| format!("failed to write Markdown {}: {err}", path.display()))
        }
    }
}

fn export_subtitle(source: ReportSource, flake: &str) -> String {
    match source {
        ReportSource::Demo => format!("Showing #{flake} sample"),
        _ => format!("{} #{flake} flake", source.label()),
    }
}

fn default_file_name(flake: &str, format: ExportFormat) -> String {
    format!(
        "sunix-report-{}.{}",
        sanitize_file_part(flake),
        format.extension()
    )
}

fn default_export_path(flake: &str, format: ExportFormat) -> String {
    default_export_dir()
        .join(default_file_name(flake, format))
        .display()
        .to_string()
}

fn default_export_dir() -> PathBuf {
    if let Some(download_dir) = non_empty_env_path("XDG_DOWNLOAD_DIR") {
        return download_dir;
    }

    if let Some(home) = home_dir() {
        let downloads = home.join("Downloads");
        if downloads.is_dir() {
            return downloads;
        }
        return home;
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn non_empty_env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn home_dir() -> Option<PathBuf> {
    non_empty_env_path("HOME")
}

fn sanitize_file_part(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "report".to_owned()
    } else {
        sanitized
    }
}

fn export_path_from_entry(
    text: &str,
    flake: &str,
    format: ExportFormat,
) -> Result<PathBuf, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Enter an export path".to_owned());
    }

    let path = expand_home_path(trimmed);
    if path.is_dir() {
        Ok(path.join(default_file_name(flake, format)))
    } else {
        Ok(with_extension(path, format))
    }
}

fn expand_home_path(value: &str) -> PathBuf {
    let Some(home) = home_dir() else {
        return PathBuf::from(value);
    };

    if value == "~" || value == "$HOME" || value == "${HOME}" {
        return home;
    }

    for prefix in ["~/", "$HOME/", "${HOME}/"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            return home.join(rest);
        }
    }

    PathBuf::from(value)
}

fn with_extension(mut path: PathBuf, format: ExportFormat) -> PathBuf {
    let expected = format.extension();
    let has_expected = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case(expected))
        .unwrap_or(false);

    if !has_expected {
        path.set_extension(expected);
    }

    path
}

fn short_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

#[derive(Clone, Copy)]
enum StatusTone {
    Neutral,
    Success,
    Error,
}

fn set_status(status: &gtk::Label, message: &str, tone: StatusTone) {
    status.set_text(message);
    status.remove_css_class("success");
    status.remove_css_class("error");

    if message.is_empty() {
        return;
    }

    match tone {
        StatusTone::Neutral => {}
        StatusTone::Success => status.add_css_class("success"),
        StatusTone::Error => status.add_css_class("error"),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExportFormat {
    Pdf,
    Markdown,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Markdown => "md",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Pdf => "PDF",
            Self::Markdown => "Markdown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExportFormat, default_file_name, export_path_from_entry, sanitize_file_part, with_extension,
    };

    #[test]
    fn sanitizes_file_parts() {
        assert_eq!(sanitize_file_part("niri-hdmi"), "niri-hdmi");
        assert_eq!(sanitize_file_part("nixos/aorus"), "nixos-aorus");
        assert_eq!(sanitize_file_part(""), "report");
    }

    #[test]
    fn replaces_missing_or_mismatched_extensions() {
        assert_eq!(
            with_extension("report.txt".into(), ExportFormat::Pdf),
            std::path::PathBuf::from("report.pdf")
        );
        assert_eq!(
            with_extension("report.PDF".into(), ExportFormat::Pdf),
            std::path::PathBuf::from("report.PDF")
        );
        assert_eq!(
            with_extension("report.txt".into(), ExportFormat::Markdown),
            std::path::PathBuf::from("report.md")
        );
    }

    #[test]
    fn uses_sunix_report_file_name_defaults() {
        assert_eq!(
            default_file_name("sample", ExportFormat::Markdown),
            "sunix-report-sample.md"
        );
        assert_eq!(
            default_file_name("nixos/aorus", ExportFormat::Pdf),
            "sunix-report-nixos-aorus.pdf"
        );
    }

    #[test]
    fn rejects_empty_export_paths() {
        assert_eq!(
            export_path_from_entry("  ", "sample", ExportFormat::Pdf),
            Err("Enter an export path".to_owned())
        );
    }

    #[test]
    fn resolves_export_path_extensions() {
        assert_eq!(
            export_path_from_entry("report.txt", "sample", ExportFormat::Pdf)
                .expect("path should resolve"),
            std::path::PathBuf::from("report.pdf")
        );
    }

    #[test]
    fn resolves_directory_export_paths() {
        assert_eq!(
            export_path_from_entry(
                std::env::temp_dir().to_string_lossy().as_ref(),
                "sample/report",
                ExportFormat::Pdf
            )
            .expect("directory should resolve"),
            std::env::temp_dir().join("sunix-report-sample-report.pdf")
        );
        assert_eq!(
            export_path_from_entry(
                std::env::temp_dir().to_string_lossy().as_ref(),
                "sample/report",
                ExportFormat::Markdown
            )
            .expect("directory should resolve"),
            std::env::temp_dir().join("sunix-report-sample-report.md")
        );
    }
}
