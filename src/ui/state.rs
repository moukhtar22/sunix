use std::cell::{Cell, RefCell};

use gtk::prelude::*;

use crate::config::SunixConfig;
use crate::model::UpdateReport;

#[derive(Clone)]
pub(super) struct AppState {
    config: Result<SunixConfig, String>,
    view: Cell<ViewState>,
    scroll_adjustment: RefCell<Option<gtk::Adjustment>>,
    report_export_button: RefCell<Option<gtk::Button>>,
    report_export_popover: RefCell<Option<gtk::Popover>>,
    home_report_cache: RefCell<Option<UpdateReport>>,
    nixos_report_cache: RefCell<Option<UpdateReport>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ReportSource {
    HomeManager,
    NixOS,
    Demo,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ViewState {
    Chooser,
    Loading,
    Report,
    Message,
}

impl AppState {
    pub(super) fn new(config: Result<SunixConfig, String>) -> Self {
        Self {
            config,
            view: Cell::new(ViewState::Chooser),
            scroll_adjustment: RefCell::new(None),
            report_export_button: RefCell::new(None),
            report_export_popover: RefCell::new(None),
            home_report_cache: RefCell::new(None),
            nixos_report_cache: RefCell::new(None),
        }
    }

    pub(super) fn config_error(&self) -> Option<&str> {
        self.config.as_ref().err().map(String::as_str)
    }

    pub(super) fn clone_config(&self) -> Result<SunixConfig, String> {
        self.config.clone()
    }

    pub(super) fn view(&self) -> ViewState {
        self.view.get()
    }

    pub(super) fn set_view(&self, view: ViewState) {
        self.view.set(view);
        self.report_export_button.replace(None);
        self.report_export_popover.replace(None);
    }

    pub(super) fn clear_scroll_adjustment(&self) {
        self.scroll_adjustment.replace(None);
    }

    pub(super) fn register_scroll_adjustment(&self, scroller: &gtk::ScrolledWindow) {
        self.scroll_adjustment.replace(Some(scroller.vadjustment()));
    }

    pub(super) fn scroll_active_view(&self, direction: f64) -> bool {
        if !matches!(self.view(), ViewState::Report | ViewState::Message) {
            return false;
        }

        let Some(adjustment) = self.scroll_adjustment.borrow().clone() else {
            return false;
        };

        let step = adjustment.step_increment().max(72.0);
        let lower = adjustment.lower();
        let upper = (adjustment.upper() - adjustment.page_size()).max(lower);
        let next_value = (adjustment.value() + (step * direction)).clamp(lower, upper);
        adjustment.set_value(next_value);
        true
    }

    pub(super) fn register_report_export_controls(
        &self,
        button: &gtk::Button,
        popover: &gtk::Popover,
    ) {
        self.report_export_button.replace(Some(button.clone()));
        self.report_export_popover.replace(Some(popover.clone()));
    }

    pub(super) fn activate_report_export(&self) -> bool {
        if self.view() != ViewState::Report {
            return false;
        }

        let Some(button) = self.report_export_button.borrow().clone() else {
            return false;
        };

        button.emit_clicked();
        true
    }

    pub(super) fn close_report_export(&self) -> bool {
        let Some(popover) = self.report_export_popover.borrow().clone() else {
            return false;
        };

        if popover.parent().is_none() {
            return false;
        }

        popover.popdown();
        true
    }

    pub(super) fn show_demo_enabled(&self) -> bool {
        self.config
            .as_ref()
            .map(|config| config.show_demo)
            .unwrap_or(false)
    }

    pub(super) fn cached_report(&self, source: ReportSource) -> Option<UpdateReport> {
        match source {
            ReportSource::HomeManager => self.home_report_cache.borrow().clone(),
            ReportSource::NixOS => self.nixos_report_cache.borrow().clone(),
            ReportSource::Demo => None,
        }
    }

    pub(super) fn cache_report(&self, source: ReportSource, report: UpdateReport) {
        match source {
            ReportSource::HomeManager => self.home_report_cache.replace(Some(report)),
            ReportSource::NixOS => self.nixos_report_cache.replace(Some(report)),
            ReportSource::Demo => None,
        };
    }
}

impl ReportSource {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::HomeManager => "Home Manager",
            Self::NixOS => "NixOS",
            Self::Demo => "Demo",
        }
    }

    pub(super) fn flake(self, config: &SunixConfig) -> &str {
        match self {
            Self::HomeManager => &config.home_flake,
            Self::NixOS => &config.nixos_flake,
            Self::Demo => "demo",
        }
    }

    pub(super) fn disconnected_message(self) -> &'static str {
        match self {
            Self::HomeManager => "Home Manager update report worker error",
            Self::NixOS => "NixOS update report worker error",
            Self::Demo => "Demo update report worker error",
        }
    }
}
