use serde::Deserialize;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone, Debug, Deserialize)]
pub struct TracingLoggerConfig {
    level: String,
    thread_ids: bool,
    thread_names: bool,
    line_number: bool,
    ansi: bool,
    filter: Option<String>,
    format: String,
    file: bool,
}

impl TracingLoggerConfig {
    fn level_directive(&self) -> &'static str {
        match self.level.to_lowercase().trim() {
            "trace" => "trace",
            "debug" => "debug",
            "warn" => "warn",
            "error" => "error",
            "off" => "off",
            _ => "info",
        }
    }

    fn normalized_filter(&self) -> Option<String> {
        self.filter
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }
}

pub fn setup_tracing(config: TracingLoggerConfig) {
    let filter_directive = config
        .normalized_filter()
        .unwrap_or_else(|| config.level_directive().to_string());

    let env_filter = EnvFilter::try_new(filter_directive).unwrap_or_else(|err| {
        eprintln!("Invalid logging.filter. Falling back to logging.level. Error: {err}");
        EnvFilter::new(config.level_directive())
    });

    let builder = fmt()
        .with_thread_ids(config.thread_ids)
        .with_thread_names(config.thread_names)
        .with_line_number(config.line_number)
        .with_ansi(config.ansi)
        .with_file(config.file)
        .with_env_filter(env_filter);

    match config.format.as_str() {
        "json" => builder.json().init(),
        "compact" => builder.compact().init(),
        "pretty" => builder.pretty().init(),
        _ => builder.init(),
    };
}
