use gtk::prelude::*;

pub(super) fn chooser_button(text: &str) -> gtk::Button {
    let button = gtk::Button::with_label(text);
    button.add_css_class("chooser-button");
    button.set_focus_on_click(false);
    button
}

pub(super) fn header_icon_button(icon_name: &str, tooltip: &str) -> gtk::Button {
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(20);
    icon.set_size_request(20, 20);
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);

    let button = gtk::Button::new();
    button.add_css_class("header-icon-button");
    button.set_focus_on_click(false);
    button.set_size_request(44, 44);
    button.set_tooltip_text(Some(tooltip));
    button.set_child(Some(&icon));
    button
}

pub(super) fn chooser_key_hints(show_demo: bool) -> gtk::Box {
    let mut hints = vec![("M", "Home Manager"), ("N", "NixOS")];
    if show_demo {
        hints.push(("D", "Demo"));
    }
    hints.push(("Esc", "Close"));
    key_hints(&hints)
}

pub(super) fn report_key_hints(show_demo: bool, show_export: bool) -> gtk::Box {
    let mut hints = vec![
        ("↑ / K", "Scroll Up"),
        ("↓ / J", "Scroll Down"),
        ("M", "Home Manager"),
        ("N", "NixOS"),
    ];
    if show_demo {
        hints.push(("D", "Demo"));
    }
    if show_export {
        hints.push(("E", "Export"));
    }
    hints.push(("← / H", "Back"));
    hints.push(("Esc", "Close"));
    key_hints(&hints)
}

pub(super) fn export_key_hints() -> gtk::Box {
    key_hints(&[("Alt+P", "PDF"), ("Alt+M", "Markdown")])
}

pub(super) fn close_key_hints() -> gtk::Box {
    key_hints(&[("Esc", "Close")])
}

pub(super) fn label(text: &str, classes: &[&str], xalign: f32) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(xalign);
    label.set_halign(gtk::Align::Fill);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);

    for class in classes {
        label.add_css_class(class);
    }

    label
}

pub(super) fn message_label(text: &str, classes: &[&str], xalign: f32) -> gtk::Label {
    let label = label(text, classes, xalign);
    label.set_ellipsize(gtk::pango::EllipsizeMode::None);
    label.set_selectable(true);
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label
}

pub(super) fn clear_view(window: &gtk::ApplicationWindow, container: &gtk::Box) {
    gtk::prelude::RootExt::set_focus(window, None::<&gtk::Widget>);
    clear(container);
}

fn key_hints(hints: &[(&str, &str)]) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    container.add_css_class("key-hints");
    container.set_halign(gtk::Align::Center);

    for (key, description) in hints {
        let hint = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        hint.add_css_class("key-hint");
        hint.append(&label(key, &["keycap"], 0.5));
        hint.append(&label(description, &["key-hint-label"], 0.0));
        container.append(&hint);
    }

    container
}

fn clear(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}
