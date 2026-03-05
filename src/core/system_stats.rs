use chrono::{DateTime, Utc};

pub struct SystemStats {
    pub report_count: u32,
    pub last_report: DateTime<Utc>,
    pub expired: bool,
    pub last_intel_report: String,
}

impl SystemStats {
    pub fn new() -> Self {
        Self {
            report_count: 1,
            last_report: Utc::now(),
            expired: false,
            last_intel_report: String::new(),
        }
    }

    pub fn update(&mut self, intel_report: Option<&str>) {
        self.last_report = Utc::now();
        self.report_count += 1;
        self.expired = false;
        if let Some(report) = intel_report {
            self.last_intel_report = report.to_string();
        }
    }
}
