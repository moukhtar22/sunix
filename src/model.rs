#[derive(Clone, Debug)]
pub struct UpdateReport {
    pub flake: String,
    pub groups: Vec<ChangeGroup>,
    pub totals: ReportTotals,
}

#[derive(Clone, Debug)]
pub struct PackageChange {
    pub name: String,
    pub version: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub size: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VersionChange {
    pub old: Option<String>,
    pub new: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ChangeGroup {
    pub status: ChangeStatus,
    pub items: Vec<PackageChange>,
}

#[derive(Clone, Debug, Default)]
pub struct ReportTotals {
    pub paths: Option<PathSummary>,
    pub size_old: Option<i64>,
    pub size_new: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct PathSummary {
    pub old: i64,
    pub new: i64,
    pub added: i64,
    pub removed: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangeStatus {
    Added,
    Removed,
    Upgraded,
    Downgraded,
    Changed,
    Other(String),
}

impl ChangeStatus {
    pub fn from_dix(status: String) -> Self {
        match status.as_str() {
            "Added" => Self::Added,
            "Removed" => Self::Removed,
            "Upgraded" => Self::Upgraded,
            "Downgraded" => Self::Downgraded,
            "Changed" => Self::Changed,
            _ => Self::Other(status),
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Upgraded => "upgraded",
            Self::Downgraded => "downgraded",
            Self::Changed => "changed",
            Self::Other(_) => "other",
        }
    }

    pub fn default_marker(&self) -> &'static str {
        match self {
            Self::Added => "[A+]",
            Self::Removed => "[R]",
            Self::Upgraded => "[U.]",
            Self::Downgraded => "[D]",
            Self::Changed => "[C]",
            Self::Other(_) => "[?]",
        }
    }

    pub fn heading(&self) -> &str {
        match self {
            Self::Added => "Added",
            Self::Removed => "Removed",
            Self::Upgraded => "Upgraded",
            Self::Downgraded => "Downgraded",
            Self::Changed => "Changed",
            Self::Other(status) => status,
        }
    }

    pub fn summary_label(&self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Upgraded => "upgraded",
            Self::Downgraded => "downgraded",
            Self::Changed => "changed",
            Self::Other(_) => "other",
        }
    }

    pub fn has_old_new_versions(&self) -> bool {
        matches!(self, Self::Upgraded | Self::Downgraded)
    }

    pub fn order(&self) -> u8 {
        match self {
            Self::Added => 0,
            Self::Removed => 1,
            Self::Upgraded => 2,
            Self::Downgraded => 3,
            Self::Changed => 4,
            Self::Other(_) => 5,
        }
    }
}
