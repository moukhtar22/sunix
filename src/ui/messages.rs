use std::rc::Rc;

use gtk::prelude::*;

use super::navigation::back_button;
use super::state::{AppState, ReportSource, ViewState};
use super::widgets::{clear_view, close_key_hints, label, message_label, report_key_hints};
use super::{
    APP_TITLE, MESSAGE_MIN_WINDOW_HEIGHT, MESSAGE_MIN_WINDOW_WIDTH, MESSAGE_WINDOW_HEIGHT,
    MESSAGE_WINDOW_WIDTH,
};

pub(super) fn show_config_error(window: &gtk::ApplicationWindow, root: &gtk::Box, message: &str) {
    show_message(
        window,
        root,
        APP_TITLE,
        message,
        &["message-body", "error"],
        None,
    );
}

pub(super) fn show_report_error(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<AppState>,
    message: &str,
) {
    show_message(
        window,
        root,
        APP_TITLE,
        message,
        &["message-body", "error"],
        Some(state),
    );
}

pub(super) fn show_loading_message(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    title: &str,
    source: ReportSource,
    flake: &str,
) -> LoadingLog {
    window.set_default_size(MESSAGE_WINDOW_WIDTH, MESSAGE_WINDOW_HEIGHT);
    window.set_size_request(MESSAGE_MIN_WINDOW_WIDTH, MESSAGE_MIN_WINDOW_HEIGHT);

    clear_view(window, root);
    root.remove_css_class("chooser-root");
    root.add_css_class("message-root");

    let container = gtk::Box::new(gtk::Orientation::Vertical, 18);
    container.add_css_class("message");
    container.add_css_class("loading-message");
    container.set_vexpand(true);
    container.set_valign(gtk::Align::Center);

    let title = label(title, &["title", "message-title", "loading-title"], 0.5);
    title.set_halign(gtk::Align::Center);
    container.append(&title);

    let spinner = gtk::Spinner::new();
    spinner.add_css_class("loading-spinner");
    spinner.set_halign(gtk::Align::Center);
    spinner.set_valign(gtk::Align::Center);
    spinner.start();
    container.append(&spinner);

    container.append(&loading_message(source, flake));
    let log = loading_log();
    container.append(log.widget());

    root.append(&container);
    root.append(&close_key_hints());
    root.queue_resize();

    log
}

fn loading_message(source: ReportSource, flake: &str) -> gtk::Box {
    let message = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    message.add_css_class("loading-text");
    message.set_halign(gtk::Align::Center);
    message.append(&label(
        "Building ",
        &["message-body", "loading", "loading-prefix"],
        0.0,
    ));
    message.append(&label(
        &format!("#{flake}"),
        &["message-body", "loading", "flake-tag"],
        0.0,
    ));
    message.append(&label(
        &format!(" {} configuration...", source.label()),
        &["message-body", "loading", "loading-suffix"],
        0.0,
    ));
    message
}

#[derive(Clone)]
pub(super) struct LoadingLog {
    container: gtk::Box,
    lines: Vec<gtk::Label>,
}

impl LoadingLog {
    pub(super) fn set_lines(&self, lines: &[&str]) {
        for (index, label) in self.lines.iter().enumerate() {
            label.set_text(lines.get(index).copied().unwrap_or(""));
        }
    }

    fn widget(&self) -> &gtk::Box {
        &self.container
    }
}

fn loading_log() -> LoadingLog {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    container.add_css_class("loading-log");
    container.set_halign(gtk::Align::Fill);
    container.set_hexpand(true);
    container.set_overflow(gtk::Overflow::Hidden);

    let mut lines = Vec::with_capacity(6);
    for _ in 0..6 {
        let line = label("", &["loading-log-line"], 0.0);
        line.set_ellipsize(gtk::pango::EllipsizeMode::End);
        line.set_lines(1);
        line.set_max_width_chars(72);
        line.set_single_line_mode(true);
        line.set_wrap(false);
        container.append(&line);
        lines.push(line);
    }

    LoadingLog { container, lines }
}

fn show_message(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    title: &str,
    message: &str,
    body_classes: &[&str],
    back_state: Option<Rc<AppState>>,
) {
    window.set_default_size(MESSAGE_WINDOW_WIDTH, MESSAGE_WINDOW_HEIGHT);
    window.set_size_request(MESSAGE_MIN_WINDOW_WIDTH, MESSAGE_MIN_WINDOW_HEIGHT);

    clear_view(window, root);
    root.remove_css_class("chooser-root");
    root.add_css_class("message-root");

    let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
    container.add_css_class("message");
    container.set_vexpand(true);
    container.append(&label(title, &["title", "message-title"], 0.0));

    let body = message_label(message, body_classes, 0.0);
    let scroller = gtk::ScrolledWindow::new();
    scroller.add_css_class("message-scroller");
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroller.set_min_content_height(120);
    scroller.set_max_content_height(420);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&body));
    container.append(&scroller);

    if let Some(state) = back_state {
        state.set_view(ViewState::Message);
        state.register_scroll_adjustment(&scroller);
        let show_demo = state.show_demo_enabled();
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        actions.add_css_class("message-actions");
        actions.set_halign(gtk::Align::End);
        actions.append(&back_button(window, root, state));
        container.append(&actions);
        root.append(&container);
        root.append(&report_key_hints(show_demo, false));
    } else {
        root.append(&container);
        root.append(&close_key_hints());
    }
    root.queue_resize();
}
