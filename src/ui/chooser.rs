use std::rc::Rc;

use gtk::prelude::*;

use super::navigation::connect_report_button;
use super::state::{AppState, ReportSource, ViewState};
use super::widgets::{chooser_button, chooser_key_hints, clear_view, label};
use super::{
    APP_TITLE, CHOOSER_MIN_WINDOW_HEIGHT, CHOOSER_MIN_WINDOW_WIDTH, CHOOSER_WINDOW_HEIGHT,
    CHOOSER_WINDOW_WIDTH,
};

pub(super) fn show_chooser(window: &gtk::ApplicationWindow, root: &gtk::Box, state: Rc<AppState>) {
    let height = if state.show_demo_enabled() {
        CHOOSER_WINDOW_HEIGHT + 80
    } else {
        CHOOSER_WINDOW_HEIGHT
    };
    let min_height = if state.show_demo_enabled() {
        CHOOSER_MIN_WINDOW_HEIGHT + 80
    } else {
        CHOOSER_MIN_WINDOW_HEIGHT
    };
    window.set_default_size(CHOOSER_WINDOW_WIDTH, height);
    window.set_size_request(CHOOSER_MIN_WINDOW_WIDTH, min_height);

    state.set_view(ViewState::Chooser);
    state.clear_scroll_adjustment();
    clear_view(window, root);
    root.remove_css_class("message-root");
    root.add_css_class("chooser-root");
    root.append(&chooser(window, root, state));
    root.queue_resize();
}

fn chooser(window: &gtk::ApplicationWindow, root: &gtk::Box, state: Rc<AppState>) -> gtk::Box {
    let chooser = gtk::Box::new(gtk::Orientation::Vertical, 0);
    chooser.add_css_class("chooser");
    chooser.set_vexpand(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 24);
    content.add_css_class("chooser-content");
    content.set_vexpand(true);
    content.set_valign(gtk::Align::Center);

    let title = label(APP_TITLE, &["title", "chooser-title"], 0.5);
    content.append(&title);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    buttons.add_css_class("chooser-buttons");
    buttons.set_halign(gtk::Align::Center);
    let show_demo = state.show_demo_enabled();

    let home_manager = chooser_button("Home Manager");
    let nixos = chooser_button("NixOS");

    connect_report_button(
        &home_manager,
        window,
        root,
        Rc::clone(&state),
        ReportSource::HomeManager,
    );
    connect_report_button(&nixos, window, root, Rc::clone(&state), ReportSource::NixOS);

    buttons.append(&home_manager);
    buttons.append(&nixos);
    content.append(&buttons);

    if show_demo {
        let demo = chooser_button("Demo");
        demo.add_css_class("demo-button");
        demo.set_halign(gtk::Align::Center);
        connect_report_button(&demo, window, root, Rc::clone(&state), ReportSource::Demo);
        content.append(&demo);
    }

    chooser.append(&content);
    chooser.append(&chooser_key_hints(show_demo));

    chooser
}
