use serde::Deserialize;
use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone, Debug, Deserialize)]
pub struct TracingLoggerConfig {
    level: String,
    thread_ids: bool,
    thread_names: bool,
    line_number: bool,
    ansi: bool,
    filter: String,
    format: String,
    file: bool,
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
    let builder = fmt()
        .with_max_level(config.level())
        .with_thread_ids(config.thread_ids)
        .with_thread_names(config.thread_names)
        .with_line_number(config.line_number)
        .with_ansi(config.ansi)
        .with_file(config.file)
        .with_env_filter(EnvFilter::new(config.filter));

    match config.format.as_str() {
        "json" => builder.json().init(),
        "compact" => builder.compact().init(),
        "pretty" => builder.pretty().init(),
        _ => builder.init(),
    };
}
