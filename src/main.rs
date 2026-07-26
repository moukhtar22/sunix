use gtk::glib;

mod cli;
mod config;
mod dix;
mod format;
mod model;
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

    default_to_gl_renderer();

    let config = config::load_config().map(|mut config| {
        config.show_demo |= options.demo;
        config
    });

    ui::run(config)
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
