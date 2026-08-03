use std::fs;
use std::rc::Rc;

use gtk::gdk;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::config::SunixConfig;

mod chooser;
mod export;
mod messages;
mod navigation;
mod report;
mod state;
mod widgets;

use chooser::show_chooser;
use messages::show_config_error;
use navigation::connect_keyboard_shortcuts;
use state::AppState;

const APP_ID: &str = "com.gvolpe.Sunix";
const APP_TITLE: &str = "Software Updates for Nix (SUNix)";
const CHOOSER_MIN_WINDOW_WIDTH: i32 = 520;
const CHOOSER_MIN_WINDOW_HEIGHT: i32 = 220;
const CHOOSER_WINDOW_WIDTH: i32 = 720;
const CHOOSER_WINDOW_HEIGHT: i32 = 260;
const MESSAGE_MIN_WINDOW_WIDTH: i32 = 520;
const MESSAGE_MIN_WINDOW_HEIGHT: i32 = 260;
const MESSAGE_WINDOW_WIDTH: i32 = 720;
const MESSAGE_WINDOW_HEIGHT: i32 = 360;
const REPORT_MIN_WINDOW_WIDTH: i32 = 960;
const REPORT_MIN_WINDOW_HEIGHT: i32 = 760;
const REPORT_WINDOW_WIDTH: i32 = 1040;
const REPORT_WINDOW_HEIGHT: i32 = 980;
const CSS: &str = include_str!("../assets/style.css");

pub fn run(config: Result<SunixConfig, String>) -> gtk::glib::ExitCode {
    let (config, style_css) = prepare_style_css(config);
    let app = gtk::Application::builder().application_id(APP_ID).build();
    let state = Rc::new(AppState::new(config));

    app.connect_startup(move |_| load_css(&style_css));
    app.connect_activate(move |app| build_ui(app, Rc::clone(&state)));

    app.run_with_args(&["sunix"])
}

#[derive(Clone, Debug)]
enum StyleCss {
    Default,
    Custom(String),
}

fn prepare_style_css(
    config: Result<SunixConfig, String>,
) -> (Result<SunixConfig, String>, StyleCss) {
    match config {
        Ok(config) => {
            let Some(path) = config.style_css.clone() else {
                return (Ok(config), StyleCss::Default);
            };

            match fs::read_to_string(&path) {
                Ok(css) => (Ok(config), StyleCss::Custom(css)),
                Err(err) => (
                    Err(format!(
                        "failed to read style_css {}: {err}",
                        path.display()
                    )),
                    StyleCss::Default,
                ),
            }
        }
        Err(message) => (Err(message), StyleCss::Default),
    }
}

fn load_css(style_css: &StyleCss) {
    let provider = gtk::CssProvider::new();
    let css = match style_css {
        StyleCss::Default => CSS,
        StyleCss::Custom(css) => css,
    };

    #[allow(deprecated)]
    provider.load_from_data(css);

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_ui(app: &gtk::Application, state: Rc<AppState>) {
    let window = gtk::ApplicationWindow::new(app);
    window.set_title(Some(APP_TITLE));
    window.set_decorated(false);
    window.set_resizable(true);
    window.set_default_size(CHOOSER_WINDOW_WIDTH, CHOOSER_WINDOW_HEIGHT);
    window.set_size_request(CHOOSER_MIN_WINDOW_WIDTH, CHOOSER_MIN_WINDOW_HEIGHT);
    window.add_css_class("update-popup");

    window.init_layer_shell();
    window.set_namespace(Some("sunix"));
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_anchor(Edge::Top, true);
    window.set_margin(Edge::Top, 16);
    window.set_exclusive_zone(0);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("updates-root");

    if let Some(message) = state.config_error() {
        show_config_error(&window, &root, message);
    } else {
        show_chooser(&window, &root, Rc::clone(&state));
    }

    connect_keyboard_shortcuts(&window, &root, state);

    window.set_child(Some(&root));
    window.present();
}
