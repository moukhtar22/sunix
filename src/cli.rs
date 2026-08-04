#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CliOptions {
    pub demo: bool,
    pub help: bool,
    pub report: Option<CliReport>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CliReport {
    pub format: CliReportFormat,
    pub title: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CliReportFormat {
    Markdown,
    Pdf,
}

impl CliOptions {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            demo: false,
            help: false,
            report: None,
        };

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--demo" => options.demo = true,
                "-h" | "--help" => options.help = true,
                "--markdown-report" => {
                    options.set_report(CliReportFormat::Markdown, &arg, args.next())?;
                }
                "--pdf-report" => {
                    options.set_report(CliReportFormat::Pdf, &arg, args.next())?;
                }
                _ => return Err(format!("sunix: unsupported argument `{arg}`")),
            }
        }

        if options.report.is_some() && options.demo {
            return Err("sunix: --demo cannot be used with report output flags".to_owned());
        }

        Ok(options)
    }

    fn set_report(
        &mut self,
        format: CliReportFormat,
        flag: &str,
        title: Option<String>,
    ) -> Result<(), String> {
        if self.report.is_some() {
            return Err("sunix: only one report output flag can be used".to_owned());
        }

        let title =
            title.ok_or_else(|| format!("sunix: {flag} requires a report title argument"))?;
        if title.starts_with('-') {
            return Err(format!("sunix: {flag} requires a report title argument"));
        }
        if title.trim().is_empty() {
            return Err(format!("sunix: {flag} requires a non-empty report title"));
        }

        self.report = Some(CliReport { format, title });
        Ok(())
    }
}

pub fn usage() -> &'static str {
    "Usage: sunix [--demo]\n       sunix --markdown-report <title>\n       sunix --pdf-report <title>\n\nOptions:\n  --demo                     Show the Demo button regardless of show_demo config\n  --markdown-report <title>  Read dix JSON from stdin and write a Markdown report to stdout\n  --pdf-report <title>       Read dix JSON from stdin and write a PDF report to stdout\n  -h, --help                 Show this help"
}

#[cfg(test)]
mod tests {
    use super::{CliOptions, CliReport, CliReportFormat, usage};

    #[test]
    fn parses_no_options() {
        assert_eq!(
            CliOptions::parse([]).unwrap(),
            CliOptions {
                demo: false,
                help: false,
                report: None,
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
                report: None,
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
                report: None,
            }
        );
        assert_eq!(
            CliOptions::parse(["-h".to_string()]).unwrap(),
            CliOptions {
                demo: false,
                help: true,
                report: None,
            }
        );
    }

    #[test]
    fn parses_markdown_report_option() {
        assert_eq!(
            CliOptions::parse(["--markdown-report".to_string(), "NixOS .#aorus".to_string()])
                .unwrap(),
            CliOptions {
                demo: false,
                help: false,
                report: Some(CliReport {
                    format: CliReportFormat::Markdown,
                    title: "NixOS .#aorus".to_owned(),
                }),
            }
        );
    }

    #[test]
    fn parses_pdf_report_option() {
        assert_eq!(
            CliOptions::parse([
                "--pdf-report".to_string(),
                "Home Manager .#niri".to_string()
            ])
            .unwrap(),
            CliOptions {
                demo: false,
                help: false,
                report: Some(CliReport {
                    format: CliReportFormat::Pdf,
                    title: "Home Manager .#niri".to_owned(),
                }),
            }
        );
    }

    #[test]
    fn rejects_report_without_title() {
        assert!(CliOptions::parse(["--markdown-report".to_string()]).is_err());
        assert!(CliOptions::parse(["--pdf-report".to_string()]).is_err());
        assert!(
            CliOptions::parse(["--markdown-report".to_string(), "--demo".to_string()]).is_err()
        );
    }

    #[test]
    fn rejects_multiple_report_output_flags() {
        assert!(
            CliOptions::parse([
                "--markdown-report".to_string(),
                "NixOS".to_string(),
                "--pdf-report".to_string(),
                "NixOS".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn rejects_demo_with_report_output() {
        assert!(
            CliOptions::parse([
                "--demo".to_string(),
                "--markdown-report".to_string(),
                "NixOS".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn rejects_unknown_options() {
        assert!(CliOptions::parse(["--toggle".to_string()]).is_err());
    }

    #[test]
    fn documents_demo_usage() {
        assert!(usage().contains("--demo"));
        assert!(usage().contains("--markdown-report"));
        assert!(usage().contains("--pdf-report"));
    }
}
