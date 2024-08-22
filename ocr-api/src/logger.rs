use std::env;

pub use tracing::*;
use tracing_subscriber::{
    filter::Directive, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

pub const COMPONENT_LEVELS: &[(&str, Level)] = &[
    // Binaries
    ("ocr_api", Level::INFO),
    // Libs
    // External
    ("request", Level::INFO),
];

/// Initialize the logger
///
/// # Panics
/// Panics if the logger fails to initialize
pub fn init() {
    init_with(COMPONENT_LEVELS.to_vec());
}

#[allow(dead_code)]
pub fn init_with_app_level(level: Level) {
    let levels = COMPONENT_LEVELS
        .iter()
        .map(|(k, _v)| (k.to_owned(), level))
        .collect::<Vec<_>>();

    init_with(levels);
}

pub fn init_with<T>(levels: T)
where
    T: IntoIterator<Item = (&'static str, Level)>,
{
    let default_levels = levels
        .into_iter()
        .map(|(k, v)| {
            if k.is_empty() {
                v.to_string()
            } else {
                format!("{}={}", k, v)
            }
        })
        .fold(String::new(), |acc, a| format!("{},{}", acc, a));

    let mut base_level = EnvFilter::builder()
        .with_default_directive(Level::WARN.into())
        .parse_lossy(default_levels);

    let env_directives = env::var("OCR_API_LOG_LEVEL")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| match s.parse() {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("Failed to parse log level directive {s:?}: {e:?}");
                None
            }
        })
        .collect::<Vec<Directive>>();

    for d in env_directives {
        base_level = base_level.add_directive(d);
    }

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(base_level)
        .try_init()
        .expect("setting default subscriber failed");
}
