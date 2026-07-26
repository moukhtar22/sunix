use std::cell::Cell;
use std::rc::Rc;

use gtk::prelude::*;

use crate::format::{format_bytes, format_signed_bytes};
use crate::model::{ChangeStatus, PackageChange, ReportTotals, UpdateReport};

use super::navigation::back_button;
use super::state::{AppState, ReportSource, ViewState};
use super::widgets::{clear_view, label, report_key_hints};
use super::{
    APP_TITLE, REPORT_MIN_WINDOW_HEIGHT, REPORT_MIN_WINDOW_WIDTH, REPORT_WINDOW_HEIGHT,
    REPORT_WINDOW_WIDTH,
};

pub(super) fn show_report(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<AppState>,
    source: ReportSource,
    report: UpdateReport,
) {
    window.set_default_size(REPORT_WINDOW_WIDTH, REPORT_WINDOW_HEIGHT);
    window.set_size_request(REPORT_MIN_WINDOW_WIDTH, REPORT_MIN_WINDOW_HEIGHT);

    state.set_view(ViewState::Report);
    clear_view(window, root);
    root.remove_css_class("chooser-root");
    root.remove_css_class("message-root");

    let back = back_button(window, root, Rc::clone(&state));
    let title = label(APP_TITLE, &["title"], 0.0);
    let subtitle = report_subtitle(source, &report.flake);

    let heading = gtk::Box::new(gtk::Orientation::Vertical, 4);
    heading.set_hexpand(true);
    heading.append(&title);
    heading.append(&subtitle);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    header.add_css_class("report-header");
    header.append(&heading);
    header.append(&back);
    root.append(&header);

    let summary = gtk::Box::new(gtk::Orientation::Vertical, 0);
    summary.add_css_class("summary");

    if is_empty_report(&report) {
        summary.append(&label("No changes", &["no-changes"], 0.0));
    } else {
        let summary_counts = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        summary_counts.add_css_class("summary-counts");
        for group in &report.groups {
            summary_counts.append(&summary_count(
                &format!("{} {}", group.items.len(), group.status.summary_label()),
                &group.status,
            ));
        }
        summary.append(&summary_counts);
    }

    if let Some(metrics) = metrics_summary(&report.totals) {
        summary.append(&metrics);
    }
    root.append(&summary);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    for group in &report.groups {
        content.append(&section(&group.status, &group.items));
    }

    let scroller = gtk::ScrolledWindow::new();
    scroller.add_css_class("updates-list");
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroller.set_min_content_height(560);
    scroller.set_max_content_height(980);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&content));
    state.register_scroll_adjustment(&scroller);
    root.append(&scroller);
    root.append(&report_key_hints(state.show_demo_enabled()));
    root.append(&resize_handle(window));
}

fn is_empty_report(report: &UpdateReport) -> bool {
    report.groups.iter().all(|group| group.items.is_empty())
}

fn report_subtitle(source: ReportSource, flake: &str) -> gtk::Box {
    let subtitle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    subtitle.add_css_class("report-subtitle");
    subtitle.append(&label("└──", &["metric-branch", "subtitle-branch"], 0.0));

    let text = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    text.add_css_class("subtitle-text");
    let prefix = match source {
        ReportSource::Demo => "Showing ".to_owned(),
        _ => format!("{} ", source.label()),
    };
    let suffix = match source {
        ReportSource::Demo => " sample",
        _ => " flake",
    };
    text.append(&label(&prefix, &["subtitle", "subtitle-kind"], 0.0));
    text.append(&label(
        &format!("#{flake}"),
        &["subtitle", "flake-tag"],
        0.0,
    ));
    text.append(&label(suffix, &["subtitle", "subtitle-suffix"], 0.0));
    subtitle.append(&text);

    subtitle
}

fn resize_handle(window: &gtk::ApplicationWindow) -> gtk::Box {
    let handle = gtk::Box::new(gtk::Orientation::Vertical, 0);
    handle.add_css_class("resize-handle");
    handle.set_cursor_from_name(Some("se-resize"));
    handle.set_halign(gtk::Align::End);
    handle.set_size_request(36, 36);

    let drag = gtk::GestureDrag::builder().button(1).build();
    let initial_size = Rc::new(Cell::new((
        REPORT_MIN_WINDOW_WIDTH,
        REPORT_MIN_WINDOW_HEIGHT,
    )));

    let initial_size_for_begin = Rc::clone(&initial_size);
    let window_for_begin = window.clone();
    drag.connect_drag_begin(move |_, _, _| {
        initial_size_for_begin.set((
            window_for_begin.width().max(REPORT_MIN_WINDOW_WIDTH),
            window_for_begin.height().max(REPORT_MIN_WINDOW_HEIGHT),
        ));
    });

    let window_for_update = window.clone();
    drag.connect_drag_update(move |_, offset_x, offset_y| {
        let (initial_width, initial_height) = initial_size.get();
        let width = (initial_width as f64 + offset_x).round() as i32;
        let height = (initial_height as f64 + offset_y).round() as i32;
        window_for_update.set_default_size(
            width.max(REPORT_MIN_WINDOW_WIDTH),
            height.max(REPORT_MIN_WINDOW_HEIGHT),
        );
    });

    handle.add_controller(drag);
    handle
}

fn summary_count(text: &str, status: &ChangeStatus) -> gtk::Label {
    let count = label(text, &["summary-count", status.css_class()], 0.5);
    count.set_halign(gtk::Align::Start);
    count
}

fn metrics_summary(totals: &ReportTotals) -> Option<gtk::Box> {
    let metrics = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    metrics.add_css_class("metrics");
    metrics.append(&label("└──", &["metric-branch"], 0.0));

    let mut count = 0;

    if let Some(paths) = &totals.paths {
        metrics.append(&metric(
            "PATHS:",
            &format!(
                "{} -> {} (+{}, -{})",
                paths.old, paths.new, paths.added, paths.removed
            ),
            count,
        ));
        count += 1;
    }

    if let (Some(size_old), Some(size_new)) = (totals.size_old, totals.size_new) {
        metrics.append(&metric(
            "SIZE:",
            &format!("{} -> {}", format_bytes(size_old), format_bytes(size_new)),
            count,
        ));
        count += 1;
        metrics.append(&metric(
            "DIFF:",
            &format_signed_bytes(size_new - size_old),
            count,
        ));
        count += 1;
    }

    (count > 0).then_some(metrics)
}

fn metric(name: &str, value: &str, index: usize) -> gtk::Box {
    let metric = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    metric.add_css_class("metric");

    if index > 0 {
        metric.set_margin_start(18);
    }

    metric.append(&label(name, &["metric-label"], 0.0));
    metric.append(&label(value, &["metric-value"], 0.0));
    metric
}

fn section(status: &ChangeStatus, items: &[PackageChange]) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    container.add_css_class("section");

    let title = label(
        status.heading(),
        &["section-title", status.css_class()],
        0.0,
    );
    container.append(&title);

    if items.is_empty() {
        container.append(&label("No package changes", &["empty"], 0.0));
        return container;
    }

    let grid = gtk::Grid::new();
    grid.add_css_class("changes-grid");
    grid.set_column_spacing(12);
    grid.set_row_spacing(4);
    grid.set_column_homogeneous(false);

    attach_header(&grid, status);

    for (index, item) in items.iter().enumerate() {
        attach_item(&grid, status, item, index as i32 + 1);
    }

    container.append(&grid);
    container
}

fn attach_header(grid: &gtk::Grid, status: &ChangeStatus) {
    let headers: &[&str] = if status.has_old_new_versions() {
        &["Category", "Name", "Old Version", "New Version", "Size"]
    } else {
        &["Category", "Name", "Version", "Size"]
    };

    for (column, header) in headers.iter().enumerate() {
        let cell = label(header, &["header-cell"], 0.0);
        grid.attach(&cell, column as i32, 0, 1, 1);
    }
}

fn attach_item(grid: &gtk::Grid, status: &ChangeStatus, item: &PackageChange, row: i32) {
    let marker = label(
        status.default_marker(),
        &["status", status.css_class()],
        0.5,
    );
    marker.set_halign(gtk::Align::Start);
    grid.attach(&marker, 0, row, 1, 1);

    let name = label(&item.name, &["cell", "name-cell"], 0.0);
    name.set_hexpand(true);
    grid.attach(&name, 1, row, 1, 1);

    if status.has_old_new_versions() {
        attach_old_new_versions(grid, item, row);
        let size = label(&item.size, &["cell", "size-cell", status.css_class()], 1.0);
        grid.attach(&size, 4, row, 1, 1);
        return;
    }

    let version = label(&item.version, &["cell", "version-cell"], 0.0);
    grid.attach(&version, 2, row, 1, 1);

    let size = label(&item.size, &["cell", "size-cell", status.css_class()], 1.0);
    grid.attach(&size, 3, row, 1, 1);
}

fn attach_old_new_versions(grid: &gtk::Grid, item: &PackageChange, row: i32) {
    let old_version = label(
        item.old_version.as_deref().unwrap_or("-"),
        &["cell", "version-cell", "old-version-cell"],
        0.0,
    );
    grid.attach(&old_version, 2, row, 1, 1);

    let new_version = label(
        item.new_version.as_deref().unwrap_or("-"),
        &["cell", "version-cell", "new-version-cell"],
        0.0,
    );
    grid.attach(&new_version, 3, row, 1, 1);
}
