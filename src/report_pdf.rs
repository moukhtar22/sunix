use std::path::Path;

use cairo::{Context, FontSlant, FontWeight, PdfSurface};

use crate::model::{ChangeStatus, PackageChange, ReportTotals, UpdateReport};

const DOC_WIDTH: f64 = 1440.0;
const PDF_PAGE_HEIGHT: f64 = 1980.0;
const MARGIN: f64 = 56.0;
const TITLE_SIZE: f64 = 38.0;
const SUBTITLE_SIZE: f64 = 24.0;
const SUMMARY_SIZE: f64 = 24.0;
const METRIC_SIZE: f64 = 18.0;
const SECTION_SIZE: f64 = 28.0;
const HEADER_SIZE: f64 = 20.0;
const CELL_SIZE: f64 = 18.0;
const HEADER_HEIGHT: f64 = 42.0;
const ROW_HEIGHT: f64 = 36.0;
const SECTION_GAP: f64 = 34.0;
const SECTION_TITLE_HEIGHT: f64 = 42.0;
const CONTINUED_TITLE_HEIGHT: f64 = 34.0;
const TABLE_X: f64 = MARGIN + 16.0;
const TABLE_WIDTH: f64 = DOC_WIDTH - ((MARGIN + 16.0) * 2.0);
const CELL_PAD_X: f64 = 16.0;
const STATUS_WIDTH: f64 = 96.0;
const SIZE_WIDTH: f64 = 150.0;
const VERSION_WIDTH: f64 = 245.0;
const OLD_NEW_VERSION_WIDTH: f64 = 220.0;
const COL_GAP: f64 = 12.0;

pub fn save_report(report: &UpdateReport, subtitle: &str, path: &Path) -> Result<(), String> {
    let surface = PdfSurface::new(DOC_WIDTH, PDF_PAGE_HEIGHT, path)
        .map_err(|err| format!("failed to create PDF {}: {err}", path.display()))?;
    render_to_surface(report, subtitle, &surface)?;
    surface.finish();
    surface
        .status()
        .map_err(|err| format!("failed to write PDF {}: {err}", path.display()))
}

pub fn render_report(report: &UpdateReport, title: &str) -> Result<Vec<u8>, String> {
    let surface = PdfSurface::for_stream(DOC_WIDTH, PDF_PAGE_HEIGHT, Vec::new())
        .map_err(|err| format!("failed to create PDF output: {err}"))?;
    render_to_surface(report, &format!("SUNix Report: {title}"), &surface)?;

    let stream = surface
        .finish_output_stream()
        .map_err(|err| format!("failed to write PDF output: {}", err.error))?;
    surface
        .status()
        .map_err(|err| format!("failed to write PDF output: {err}"))?;
    stream
        .downcast::<Vec<u8>>()
        .map(|bytes| *bytes)
        .map_err(|_| "failed to recover PDF output buffer".to_owned())
}

fn render_to_surface(
    report: &UpdateReport,
    subtitle: &str,
    surface: &PdfSurface,
) -> Result<(), String> {
    let context =
        Context::new(surface).map_err(|err| format!("failed to create renderer: {err}"))?;
    let mut renderer = ReportRenderer::new(&context, PDF_PAGE_HEIGHT);
    renderer.render(report, subtitle)?;
    context
        .show_page()
        .map_err(|err| format!("failed to finish PDF page: {err}"))?;
    surface.flush();
    Ok(())
}

struct ReportRenderer<'a> {
    context: &'a Context,
    y: f64,
    page_height: f64,
}

impl<'a> ReportRenderer<'a> {
    fn new(context: &'a Context, page_height: f64) -> Self {
        Self {
            context,
            y: MARGIN,
            page_height,
        }
    }

    fn render(&mut self, report: &UpdateReport, subtitle: &str) -> Result<(), String> {
        self.draw_page_background()?;
        self.draw_header(subtitle)?;
        self.draw_summary(report)?;

        for group in report.groups.iter().filter(|group| !group.items.is_empty()) {
            self.draw_section(&group.status, &group.items)?;
        }

        Ok(())
    }

    fn draw_header(&mut self, subtitle: &str) -> Result<(), String> {
        self.draw_text(
            "Software Updates for Nix (SUNix)",
            MARGIN,
            self.y + TITLE_SIZE,
            TextStyle::sans(TITLE_SIZE, FontWeight::Bold, Color::TEXT),
            DOC_WIDTH - (MARGIN * 2.0),
        )?;
        self.y += 54.0;
        self.draw_text(
            subtitle,
            MARGIN,
            self.y + SUBTITLE_SIZE,
            TextStyle::sans(SUBTITLE_SIZE, FontWeight::Bold, Color::MUTED),
            DOC_WIDTH - (MARGIN * 2.0),
        )?;
        self.y += 58.0;
        Ok(())
    }

    fn draw_summary(&mut self, report: &UpdateReport) -> Result<(), String> {
        let mut x = MARGIN;
        let summary_y = self.y + SUMMARY_SIZE;

        for group in report.groups.iter().filter(|group| !group.items.is_empty()) {
            let text = format!("{} {}", group.items.len(), group.status.summary_label());
            let style =
                TextStyle::sans(SUMMARY_SIZE, FontWeight::Bold, status_color(&group.status));
            self.draw_text(&text, x, summary_y, style, DOC_WIDTH - MARGIN - x)?;
            x += self.text_width(&text, style)? + 30.0;
        }

        self.y += 42.0;

        if let Some(metrics) = metrics_line(&report.totals) {
            self.draw_text(
                &metrics,
                MARGIN,
                self.y + METRIC_SIZE,
                TextStyle::mono(METRIC_SIZE, FontWeight::Bold, Color::TEXT),
                DOC_WIDTH - (MARGIN * 2.0),
            )?;
            self.y += 42.0;
        }

        Ok(())
    }

    fn draw_section(
        &mut self,
        status: &ChangeStatus,
        items: &[PackageChange],
    ) -> Result<(), String> {
        let first_chunk = SECTION_GAP + SECTION_TITLE_HEIGHT + HEADER_HEIGHT + ROW_HEIGHT;
        self.ensure_space(first_chunk)?;
        self.y += SECTION_GAP;
        self.draw_section_title(status, false)?;
        self.draw_table_header(status)?;

        for item in items {
            if self.ensure_space(ROW_HEIGHT)? {
                self.draw_section_title(status, true)?;
                self.draw_table_header(status)?;
            }
            self.draw_item(status, item)?;
        }

        Ok(())
    }

    fn draw_section_title(&mut self, status: &ChangeStatus, continued: bool) -> Result<(), String> {
        let title = if continued {
            format!("{} (continued)", status.heading())
        } else {
            status.heading().to_owned()
        };
        let size = if continued { CELL_SIZE } else { SECTION_SIZE };
        let height = if continued {
            CONTINUED_TITLE_HEIGHT
        } else {
            SECTION_TITLE_HEIGHT
        };
        self.draw_text(
            &title,
            TABLE_X,
            self.y + size,
            TextStyle::sans(size, FontWeight::Bold, status_color(status)),
            TABLE_WIDTH,
        )?;
        self.y += height;
        Ok(())
    }

    fn draw_table_header(&mut self, status: &ChangeStatus) -> Result<(), String> {
        self.draw_rect(TABLE_X, self.y, TABLE_WIDTH, HEADER_HEIGHT, Color::TABLE)?;
        self.draw_border(TABLE_X, self.y, TABLE_WIDTH, HEADER_HEIGHT, Color::BORDER)?;

        let columns = columns(status);
        for column in columns {
            self.draw_text(
                column.heading,
                column.x + CELL_PAD_X,
                self.y + 27.0,
                TextStyle::sans(HEADER_SIZE, FontWeight::Bold, Color::DIM),
                column.width - (CELL_PAD_X * 2.0),
            )?;
        }

        self.y += HEADER_HEIGHT;
        Ok(())
    }

    fn draw_item(&mut self, status: &ChangeStatus, item: &PackageChange) -> Result<(), String> {
        self.draw_rect(TABLE_X, self.y, TABLE_WIDTH, ROW_HEIGHT, Color::ROW)?;
        self.draw_border(TABLE_X, self.y, TABLE_WIDTH, ROW_HEIGHT, Color::BORDER)?;

        let marker_x = TABLE_X + CELL_PAD_X;
        let marker_y = self.y + 6.0;
        self.draw_rect(marker_x, marker_y, 58.0, 24.0, status_background(status))?;
        self.draw_text(
            status.default_marker(),
            marker_x + 8.0,
            self.y + 25.0,
            TextStyle::mono(CELL_SIZE, FontWeight::Bold, status_color(status)),
            46.0,
        )?;

        let columns = columns(status);
        self.draw_text(
            &item.name,
            columns[1].x + CELL_PAD_X,
            self.y + 25.0,
            TextStyle::sans(CELL_SIZE, FontWeight::Bold, Color::TEXT),
            columns[1].width - (CELL_PAD_X * 2.0),
        )?;

        if status.has_old_new_versions() {
            self.draw_text(
                item.old_version.as_deref().unwrap_or("-"),
                columns[2].x + CELL_PAD_X,
                self.y + 25.0,
                TextStyle::mono(CELL_SIZE, FontWeight::Normal, Color::MUTED),
                columns[2].width - (CELL_PAD_X * 2.0),
            )?;
            self.draw_text(
                item.new_version.as_deref().unwrap_or("-"),
                columns[3].x + CELL_PAD_X,
                self.y + 25.0,
                TextStyle::mono(CELL_SIZE, FontWeight::Normal, Color::MUTED),
                columns[3].width - (CELL_PAD_X * 2.0),
            )?;
            self.draw_text(
                &item.size,
                columns[4].x + CELL_PAD_X,
                self.y + 25.0,
                TextStyle::mono(CELL_SIZE, FontWeight::Bold, status_color(status)),
                columns[4].width - (CELL_PAD_X * 2.0),
            )?;
        } else {
            self.draw_text(
                &item.version,
                columns[2].x + CELL_PAD_X,
                self.y + 25.0,
                TextStyle::mono(CELL_SIZE, FontWeight::Normal, Color::MUTED),
                columns[2].width - (CELL_PAD_X * 2.0),
            )?;
            self.draw_text(
                &item.size,
                columns[3].x + CELL_PAD_X,
                self.y + 25.0,
                TextStyle::mono(CELL_SIZE, FontWeight::Bold, status_color(status)),
                columns[3].width - (CELL_PAD_X * 2.0),
            )?;
        }

        self.y += ROW_HEIGHT;
        Ok(())
    }

    fn ensure_space(&mut self, needed: f64) -> Result<bool, String> {
        if self.y + needed <= self.page_height - MARGIN {
            return Ok(false);
        }

        self.context
            .show_page()
            .map_err(|err| format!("failed to finish PDF page: {err}"))?;
        self.y = MARGIN;
        self.draw_page_background()?;
        Ok(true)
    }

    fn draw_page_background(&self) -> Result<(), String> {
        self.draw_rect(0.0, 0.0, DOC_WIDTH, self.page_height, Color::BACKGROUND)
    }

    fn draw_rect(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Color,
    ) -> Result<(), String> {
        self.context
            .set_source_rgba(color.red, color.green, color.blue, color.alpha);
        self.context.rectangle(x, y, width, height);
        self.context
            .fill()
            .map_err(|err| format!("failed to fill export shape: {err}"))
    }

    fn draw_border(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Color,
    ) -> Result<(), String> {
        self.context
            .set_source_rgba(color.red, color.green, color.blue, color.alpha);
        self.context.set_line_width(1.0);
        self.context
            .rectangle(x + 0.5, y + 0.5, width - 1.0, height - 1.0);
        self.context
            .stroke()
            .map_err(|err| format!("failed to draw export border: {err}"))
    }

    fn draw_text(
        &self,
        text: &str,
        x: f64,
        baseline: f64,
        style: TextStyle,
        max_width: f64,
    ) -> Result<(), String> {
        self.set_font(style);
        let fitted = self.fit_text(text, style, max_width)?;

        self.context.set_source_rgba(
            style.color.red,
            style.color.green,
            style.color.blue,
            style.color.alpha,
        );
        self.context.move_to(x, baseline);
        self.context
            .show_text(&fitted)
            .map_err(|err| format!("failed to draw export text: {err}"))
    }

    fn fit_text(&self, text: &str, style: TextStyle, max_width: f64) -> Result<String, String> {
        if self.text_width(text, style)? <= max_width {
            return Ok(text.to_owned());
        }

        let ellipsis = "...";
        let ellipsis_width = self.text_width(ellipsis, style)?;
        if ellipsis_width > max_width {
            return Ok(String::new());
        }

        let mut fitted = String::new();
        for ch in text.chars() {
            let next = format!("{fitted}{ch}{ellipsis}");
            if self.text_width(&next, style)? > max_width {
                break;
            }
            fitted.push(ch);
        }
        fitted.push_str(ellipsis);
        Ok(fitted)
    }

    fn text_width(&self, text: &str, style: TextStyle) -> Result<f64, String> {
        self.set_font(style);
        self.context
            .text_extents(text)
            .map(|extents| extents.x_advance())
            .map_err(|err| format!("failed to measure export text: {err}"))
    }

    fn set_font(&self, style: TextStyle) {
        self.context
            .select_font_face(style.family, FontSlant::Normal, style.weight);
        self.context.set_font_size(style.size);
    }
}

#[derive(Clone, Copy)]
struct TextStyle {
    family: &'static str,
    size: f64,
    weight: FontWeight,
    color: Color,
}

impl TextStyle {
    fn sans(size: f64, weight: FontWeight, color: Color) -> Self {
        Self {
            family: "Sans",
            size,
            weight,
            color,
        }
    }

    fn mono(size: f64, weight: FontWeight, color: Color) -> Self {
        Self {
            family: "Monospace",
            size,
            weight,
            color,
        }
    }
}

#[derive(Clone, Copy)]
struct Color {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

impl Color {
    const BACKGROUND: Self = Self::rgb(25, 28, 31);
    const BORDER: Self = Self::rgba(222, 226, 230, 0.12);
    const DIM: Self = Self::rgb(141, 152, 166);
    const MUTED: Self = Self::rgb(194, 202, 211);
    const ROW: Self = Self::rgba(255, 255, 255, 0.025);
    const TABLE: Self = Self::rgba(255, 255, 255, 0.045);
    const TEXT: Self = Self::rgb(231, 235, 239);

    const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 1.0)
    }

    const fn rgba(red: u8, green: u8, blue: u8, alpha: f64) -> Self {
        Self {
            red: red as f64 / 255.0,
            green: green as f64 / 255.0,
            blue: blue as f64 / 255.0,
            alpha,
        }
    }
}

#[derive(Clone, Copy)]
struct Column {
    heading: &'static str,
    x: f64,
    width: f64,
}

fn columns(status: &ChangeStatus) -> Vec<Column> {
    let marker = Column {
        heading: "Category",
        x: TABLE_X,
        width: STATUS_WIDTH,
    };

    if status.has_old_new_versions() {
        let name_width = TABLE_WIDTH
            - STATUS_WIDTH
            - (OLD_NEW_VERSION_WIDTH * 2.0)
            - SIZE_WIDTH
            - (COL_GAP * 4.0);
        return vec![
            marker,
            Column {
                heading: "Name",
                x: TABLE_X + STATUS_WIDTH + COL_GAP,
                width: name_width,
            },
            Column {
                heading: "Old Version",
                x: TABLE_X + STATUS_WIDTH + COL_GAP + name_width + COL_GAP,
                width: OLD_NEW_VERSION_WIDTH,
            },
            Column {
                heading: "New Version",
                x: TABLE_X
                    + STATUS_WIDTH
                    + COL_GAP
                    + name_width
                    + COL_GAP
                    + OLD_NEW_VERSION_WIDTH
                    + COL_GAP,
                width: OLD_NEW_VERSION_WIDTH,
            },
            Column {
                heading: "Size",
                x: TABLE_X
                    + STATUS_WIDTH
                    + COL_GAP
                    + name_width
                    + COL_GAP
                    + OLD_NEW_VERSION_WIDTH
                    + COL_GAP
                    + OLD_NEW_VERSION_WIDTH
                    + COL_GAP,
                width: SIZE_WIDTH,
            },
        ];
    }

    let version_width = VERSION_WIDTH * 1.5;
    let name_width = TABLE_WIDTH - STATUS_WIDTH - version_width - SIZE_WIDTH - (COL_GAP * 3.0);
    vec![
        marker,
        Column {
            heading: "Name",
            x: TABLE_X + STATUS_WIDTH + COL_GAP,
            width: name_width,
        },
        Column {
            heading: "Version",
            x: TABLE_X + STATUS_WIDTH + COL_GAP + name_width + COL_GAP,
            width: version_width,
        },
        Column {
            heading: "Size",
            x: TABLE_X + STATUS_WIDTH + COL_GAP + name_width + COL_GAP + version_width + COL_GAP,
            width: SIZE_WIDTH,
        },
    ]
}

fn metrics_line(totals: &ReportTotals) -> Option<String> {
    let mut metrics = Vec::new();

    if let Some(paths) = &totals.paths {
        metrics.push(format!(
            "PATHS: {} -> {} (+{}, -{})",
            paths.old, paths.new, paths.added, paths.removed
        ));
    }

    if let (Some(size_old), Some(size_new)) = (totals.size_old, totals.size_new) {
        metrics.push(format!(
            "SIZE: {} -> {}",
            crate::format::format_bytes(size_old),
            crate::format::format_bytes(size_new)
        ));
        metrics.push(format!(
            "DIFF: {}",
            crate::format::format_signed_bytes(size_new - size_old)
        ));
    }

    (!metrics.is_empty()).then(|| metrics.join("    "))
}

fn status_color(status: &ChangeStatus) -> Color {
    match status {
        ChangeStatus::Added => Color::rgb(159, 227, 168),
        ChangeStatus::Removed => Color::rgb(255, 155, 155),
        ChangeStatus::Upgraded => Color::rgb(158, 197, 255),
        ChangeStatus::Downgraded => Color::rgb(255, 211, 158),
        ChangeStatus::Changed | ChangeStatus::Other(_) => Color::rgb(234, 217, 130),
    }
}

fn status_background(status: &ChangeStatus) -> Color {
    match status {
        ChangeStatus::Added => Color::rgba(63, 185, 106, 0.16),
        ChangeStatus::Removed => Color::rgba(224, 82, 82, 0.16),
        ChangeStatus::Upgraded => Color::rgba(82, 145, 224, 0.16),
        ChangeStatus::Downgraded => Color::rgba(224, 156, 82, 0.16),
        ChangeStatus::Changed | ChangeStatus::Other(_) => Color::rgba(214, 188, 84, 0.16),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::model::{ChangeGroup, PackageChange, ReportTotals, UpdateReport};

    use super::{render_report, save_report};

    #[test]
    fn writes_pdf_report() {
        let path = output_path();
        let _ = fs::remove_file(&path);

        save_report(&sample_report(), "NixOS #aorus flake", &path)
            .expect("PDF export should succeed");

        let bytes = fs::read(&path).expect("PDF export should write a file");
        assert!(bytes.starts_with(b"%PDF"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn renders_pdf_report_to_bytes() {
        let bytes = render_report(&sample_report(), "NixOS .#aorus")
            .expect("PDF export should render to bytes");

        assert!(bytes.starts_with(b"%PDF"));
    }

    fn output_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("sunix-export-test-{}.pdf", std::process::id()))
    }

    fn sample_report() -> UpdateReport {
        UpdateReport {
            flake: "aorus".to_owned(),
            groups: vec![ChangeGroup {
                status: crate::model::ChangeStatus::Upgraded,
                items: vec![PackageChange {
                    name: "linux".to_owned(),
                    version: "6.12.1 -> 6.12.2".to_owned(),
                    old_version: Some("6.12.1".to_owned()),
                    new_version: Some("6.12.2".to_owned()),
                    size: "+14.0 MiB".to_owned(),
                }],
            }],
            totals: ReportTotals::default(),
        }
    }
}
