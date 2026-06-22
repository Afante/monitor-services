use serde::Deserialize;
use chrono::Local;

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl From<&str> for LogLevel {
    fn from(value: &str) -> Self {
        match value {
            "error" => LogLevel::Error,
            "warning" => LogLevel::Warning,
            "info" => LogLevel::Info,
            "Debug" => LogLevel::Debug,
            "Trace" => LogLevel::Trace,
            _ => panic!("Invalid log level: {}", value)
        }
    }
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

static mut MIN_LOG_LEVEL: LogLevel = LogLevel::Info;

pub fn set_min_log_level(level: LogLevel) {
    unsafe {MIN_LOG_LEVEL = level};
}

pub fn log_line<L: Into<LogLevel>>(level: L, name: &str, msg: &str) -> String {
    let timestamp  = Local::now().format("%Y-%m-%d %H:%M:%S");
    let level: LogLevel = level.into();
    if level as u16 >= unsafe { MIN_LOG_LEVEL } as u16 {
        let line = format!("[{}][{}][{}]{}", timestamp.to_string(), level.as_str(), name, msg);
        eprintln!("{}", line);
        line
    }
    else {
        String::new()
    }
}

pub fn log_line_f<L: Into<LogLevel>, F: Fn() -> String>(level: L, name: &str, msgf: F) -> String {
    let msg = msgf();
    log_line(level, name, msg.as_str())
}