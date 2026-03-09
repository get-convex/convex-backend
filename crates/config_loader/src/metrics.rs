use std::path::Path;

use metrics::{
    register_convex_int_gauge,
    Subgauge,
};

register_convex_int_gauge!(
    CONFIG_LOADER_INVALID_CONFIG_INFO,
    "Indicates that ConfigLoader has encountered and ignored a config file parse error. If \
     nonzero, configs may be outdated or ignored.",
    &["config_file"]
);
pub(crate) fn invalid_config_gauge(config_file: &Path) -> Subgauge {
    Subgauge::new(
        CONFIG_LOADER_INVALID_CONFIG_INFO.with_label_values(&[config_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()]),
    )
}
