use serde::Deserialize;
use tracing::Level;
use tracing_subscriber::fmt;

#[derive(Clone, Debug, Deserialize)]
pub struct TracingLoggerConfig {
    level: String,
    thread_ids: bool,
    thread_names: bool,
    line_number: bool,
    ansi: bool,
}

impl TracingLoggerConfig {
    fn level(&self) -> Level {
        match self.level.to_lowercase().as_str().trim() {
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            "debug" => Level::DEBUG,
            "trace" => Level::TRACE,
            _ => Level::INFO,
        }
    }
}

pub fn setup_tracing(config: TracingLoggerConfig) {
    fmt()
        .with_max_level(config.level())
        .with_thread_ids(config.thread_ids)
        .with_thread_names(config.thread_names)
        .with_line_number(config.line_number)
        .with_ansi(config.ansi)
        .init();
}
