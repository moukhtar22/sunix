#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CliOptions {
    pub demo: bool,
    pub help: bool,
}

impl CliOptions {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            demo: false,
            help: false,
        };

        for arg in args {
            match arg.as_str() {
                "--demo" => options.demo = true,
                "-h" | "--help" => options.help = true,
                _ => return Err(format!("sunix: unsupported argument `{arg}`")),
            }
        }

        Ok(options)
    }
}

pub fn usage() -> &'static str {
    "Usage: sunix [--demo]\n\nOptions:\n  --demo      Show the Demo button regardless of show_demo config\n  -h, --help  Show this help"
}

#[cfg(test)]
mod tests {
    use super::{CliOptions, usage};

    #[test]
    fn parses_no_options() {
        assert_eq!(
            CliOptions::parse([]).unwrap(),
            CliOptions {
                demo: false,
                help: false,
            }
        );
    }

    #[test]
    fn parses_demo_option() {
        assert_eq!(
            CliOptions::parse(["--demo".to_string()]).unwrap(),
            CliOptions {
                demo: true,
                help: false,
            }
        );
    }

    #[test]
    fn parses_help_options() {
        assert_eq!(
            CliOptions::parse(["--help".to_string()]).unwrap(),
            CliOptions {
                demo: false,
                help: true,
            }
        );
        assert_eq!(
            CliOptions::parse(["-h".to_string()]).unwrap(),
            CliOptions {
                demo: false,
                help: true,
            }
        );
    }

    #[test]
    fn rejects_unknown_options() {
        assert!(CliOptions::parse(["--toggle".to_string()]).is_err());
    }

    #[test]
    fn documents_demo_usage() {
        assert!(usage().contains("--demo"));
    }
}
