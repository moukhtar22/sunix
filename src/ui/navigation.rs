use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

use gtk::gdk;
use gtk::prelude::*;

use crate::config::SunixConfig;
use crate::dix::{demo_report, home_manager_report_with_logs, nixos_report_with_logs};
use crate::model::UpdateReport;

use super::APP_TITLE;
use super::chooser::show_chooser;
use super::messages::{show_config_error, show_loading_message, show_report_error};
use super::report::show_report;
use super::state::{AppState, ReportSource, ViewState};

pub(super) fn back_button(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<AppState>,
) -> gtk::Button {
    let button = gtk::Button::from_icon_name("go-previous-symbolic");
    button.add_css_class("back-button");
    button.set_focus_on_click(false);
    button.set_halign(gtk::Align::End);
    button.set_valign(gtk::Align::Start);
    button.set_tooltip_text(Some("Back"));

    let window = window.clone();
    let root = root.clone();
    button.connect_clicked(move |_| {
        navigate_back(&window, &root, Rc::clone(&state));
    });

    button
}

pub(super) fn connect_report_button(
    button: &gtk::Button,
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<AppState>,
    source: ReportSource,
) {
    let window = window.clone();
    let root = root.clone();

    button.connect_clicked(move |_| {
        open_report_source(&window, &root, Rc::clone(&state), source);
    });
}

pub(super) fn connect_keyboard_shortcuts(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<AppState>,
) {
    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let shortcut_window = window.clone();
    let shortcut_root = root.clone();

    key_controller.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gdk::Key::Escape {
            shortcut_window.close();
            return gtk::glib::Propagation::Stop;
        }

        if keyval == gdk::Key::Left || key_matches(keyval, 'h') {
            navigate_back(&shortcut_window, &shortcut_root, Rc::clone(&state));
            return gtk::glib::Propagation::Stop;
        }

        if (keyval == gdk::Key::Up || key_matches(keyval, 'k')) && state.scroll_active_view(-1.0) {
            return gtk::glib::Propagation::Stop;
        }

        if (keyval == gdk::Key::Down || key_matches(keyval, 'j')) && state.scroll_active_view(1.0) {
            return gtk::glib::Propagation::Stop;
        }

        if key_matches(keyval, 'm') {
            open_report_source(
                &shortcut_window,
                &shortcut_root,
                Rc::clone(&state),
                ReportSource::HomeManager,
            );
            return gtk::glib::Propagation::Stop;
        }

        if key_matches(keyval, 'n') {
            open_report_source(
                &shortcut_window,
                &shortcut_root,
                Rc::clone(&state),
                ReportSource::NixOS,
            );
            return gtk::glib::Propagation::Stop;
        }

        if state.show_demo_enabled() && key_matches(keyval, 'd') {
            open_report_source(
                &shortcut_window,
                &shortcut_root,
                Rc::clone(&state),
                ReportSource::Demo,
            );
            return gtk::glib::Propagation::Stop;
        }

        gtk::glib::Propagation::Proceed
    });
    window.add_controller(key_controller);
}

fn key_matches(keyval: gdk::Key, expected: char) -> bool {
    keyval
        .to_unicode()
        .map(|value| value.eq_ignore_ascii_case(&expected))
        .unwrap_or(false)
}

fn navigate_back(window: &gtk::ApplicationWindow, root: &gtk::Box, state: Rc<AppState>) {
    if !matches!(state.view(), ViewState::Report | ViewState::Message) {
        return;
    }

    defer_view_switch(window, root, move |window, root| {
        show_chooser(window, root, state);
    });
}

fn open_report_source(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<AppState>,
    source: ReportSource,
) {
    if state.view() == ViewState::Loading {
        return;
    }

    defer_view_switch(window, root, move |window, root| {
        show_report_source(window, root, state, source);
    });
}

fn show_report_source(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<AppState>,
    source: ReportSource,
) {
    if matches!(source, ReportSource::Demo) {
        show_report(window, root, state, source, demo_report());
        return;
    }

    if let Some(report) = state.cached_report(source) {
        show_report(window, root, state, source, report);
        return;
    }

    match state.clone_config() {
        Ok(config) => show_report_loading(window, root, state, source, config),
        Err(message) => show_config_error(window, root, &message),
    }
}

fn defer_view_switch<F>(window: &gtk::ApplicationWindow, root: &gtk::Box, action: F)
where
    F: FnOnce(&gtk::ApplicationWindow, &gtk::Box) + 'static,
{
    let window = window.clone();
    let root = root.clone();
    gtk::glib::idle_add_local_once(move || action(&window, &root));
}

fn show_report_loading(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<AppState>,
    source: ReportSource,
    config: SunixConfig,
) {
    state.set_view(ViewState::Loading);
    state.clear_scroll_adjustment();
    let flake = source.flake(&config).to_owned();
    let loading_log = show_loading_message(window, root, APP_TITLE, source, &flake);

    let (report_sender, report_receiver) = mpsc::channel();
    let (log_sender, log_receiver) = mpsc::channel();
    thread::spawn(move || {
        let report = match source {
            ReportSource::HomeManager => home_manager_report_with_logs(&config, log_sender),
            ReportSource::NixOS => nixos_report_with_logs(&config, log_sender),
            ReportSource::Demo => Ok(demo_report()),
        };
        let _ = report_sender.send(report);
    });

    let window = window.downgrade();
    let root = root.downgrade();
    let mut recent_logs = RecentLogs::new(6);
    gtk::glib::timeout_add_local(Duration::from_millis(100), move || {
        let Some(window) = window.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };
        let Some(root) = root.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };

        if recent_logs.drain_from(&log_receiver) {
            loading_log.set_lines(&recent_logs.lines());
        }

        match report_receiver.try_recv() {
            Ok(Ok(report)) => {
                cache_report_and_show(&window, &root, Rc::clone(&state), source, report);
                gtk::glib::ControlFlow::Break
            }
            Ok(Err(message)) => {
                show_report_error(&window, &root, Rc::clone(&state), &message);
                gtk::glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                show_report_error(
                    &window,
                    &root,
                    Rc::clone(&state),
                    source.disconnected_message(),
                );
                gtk::glib::ControlFlow::Break
            }
        }
    });
}

fn cache_report_and_show(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<AppState>,
    source: ReportSource,
    report: UpdateReport,
) {
    state.cache_report(source, report.clone());
    show_report(window, root, state, source, report);
}

struct RecentLogs {
    lines: VecDeque<String>,
    limit: usize,
}

impl RecentLogs {
    fn new(limit: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(limit),
            limit,
        }
    }

    fn drain_from(&mut self, receiver: &mpsc::Receiver<String>) -> bool {
        let mut changed = false;

        for line in receiver.try_iter() {
            self.push(line);
            changed = true;
        }

        changed
    }

    fn push(&mut self, line: String) {
        self.lines.push_back(line);

        while self.lines.len() > self.limit {
            self.lines.pop_front();
        }
    }

    fn lines(&self) -> Vec<&str> {
        self.lines.iter().map(String::as_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::RecentLogs;

    #[test]
    fn keeps_only_the_latest_lines() {
        let mut logs = RecentLogs::new(6);

        logs.push("one".to_owned());
        logs.push("two".to_owned());
        logs.push("three".to_owned());
        logs.push("four".to_owned());
        logs.push("five".to_owned());
        logs.push("six".to_owned());
        logs.push("seven".to_owned());

        assert_eq!(
            logs.lines(),
            vec!["two", "three", "four", "five", "six", "seven"]
        );
    }
}
