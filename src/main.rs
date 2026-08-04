use std::io::{self, Read, Write};

use gtk::glib;

use crate::cli::{CliReport, CliReportFormat};

mod cli;
mod command;
mod config;
mod dix;
mod format;
mod model;
mod report_markdown;
mod report_pdf;
mod ui;

fn main() -> glib::ExitCode {
    let options = match cli::CliOptions::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("{err}");
            eprintln!("{}", cli::usage());
            return glib::ExitCode::FAILURE;
        }
    };

    if options.help {
        println!("{}", cli::usage());
        return glib::ExitCode::SUCCESS;
    }

    if let Some(report) = options.report {
        return match write_report_from_stdin(report) {
            Ok(()) => glib::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{err}");
                glib::ExitCode::FAILURE
            }
        };
    }

    default_to_gl_renderer();

    let config = config::load_config().map(|mut config| {
        config.show_demo |= options.demo;
        config
    });

    ui::run(config)
}

fn write_report_from_stdin(report_options: CliReport) -> Result<(), String> {
    let mut content = String::new();
    io::stdin()
        .read_to_string(&mut content)
        .map_err(|err| format!("sunix: failed to read dix JSON from stdin: {err}"))?;

    let report = dix::parse_report_json(&content, "dix JSON from stdin", &report_options.title)?;
    match report_options.format {
        CliReportFormat::Markdown => {
            let markdown =
                report_markdown::render_report_with_title(&report, &report_options.title);
            io::stdout()
                .lock()
                .write_all(markdown.as_bytes())
                .map_err(|err| format!("sunix: failed to write Markdown report: {err}"))
        }
        CliReportFormat::Pdf => {
            let pdf = report_pdf::render_report(&report, &report_options.title)?;
            io::stdout()
                .lock()
                .write_all(&pdf)
                .map_err(|err| format!("sunix: failed to write PDF report: {err}"))
        }
    }
}

fn default_to_gl_renderer() {
    if std::env::var_os("GSK_RENDERER").is_some() {
        return;
    }

    // SAFETY: this runs at process startup, before GTK is initialized and before
    // this program starts any threads.
    if let Err(err) = unsafe { glib::setenv("GSK_RENDERER", "gl", false) } {
        eprintln!("sunix: failed to set default GSK_RENDERER=gl: {err}");
    }
}
