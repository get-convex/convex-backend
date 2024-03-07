pub use paste::paste;
pub use prometheus;

/// Register a histogram with the Convex metrics registry and store
/// in a static variable.
/// An optional third argument allows specifying labels for this metric.
/// The reported metric name will be the lower_snake_case version of the
/// declared variable name.
#[macro_export]
macro_rules! register_convex_histogram {
    ($VIS:vis $NAME:ident, $HELP:literal $(,)?) => {
        $VIS static $NAME: std::sync::LazyLock<$crate::prometheus::VMHistogram> =
            std::sync::LazyLock::new(|| $crate::register_convex_histogram_owned!(
                $NAME,
                $HELP,
            ));
    };
    ($VIS:vis $NAME:ident, $HELP:literal, $LABELS:expr $(,)?) => {
        $VIS static $NAME: std::sync::LazyLock<$crate::prometheus::VMHistogramVec> =
            std::sync::LazyLock::new(|| $crate::register_convex_histogram_owned!(
                $NAME,
                $HELP,
                $LABELS,
            ));
    };
}

/// Register a histogram with the Convex metrics registry and return as
/// an expression.
/// An optional third argument allows specifying labels for this metric.
/// The reported metric name will be the lower_snake_case version of the
/// declared variable name.
#[macro_export]
macro_rules! register_convex_histogram_owned {
    ($NAME:ident, $HELP:literal $(,)?) => {{
        $crate::paste! {
            let name = $crate::metric_name!(stringify!([<$NAME:lower>]));
        }
        let help = $crate::metric_help!($HELP);
        #[allow(clippy::disallowed_macros)]
        $crate::prometheus::register_vmhistogram_with_registry!(
            &*name,
            &*help,
            $crate::CONVEX_METRICS_REGISTRY,
        )
        .expect("Metric initialization failed")
    }};
    ($NAME:ident, $HELP:literal, $LABELS:expr $(,)?) => {{
        $crate::paste! {
            let name = $crate::metric_name!(stringify!([<$NAME:lower>]));
        }
        let help = $crate::metric_help!($HELP);
        #[allow(clippy::disallowed_macros)]
        $crate::prometheus::register_vmhistogram_vec_with_registry!(
            &*name,
            &*help,
            $LABELS,
            $crate::CONVEX_METRICS_REGISTRY,
        )
        .expect("Metric initialization failed")
    }};
}

/// Register an integer counter with the Convex metrics registry and store
/// in a static variable.
/// An optional third argument allows specifying labels for this metric.
/// The reported metric name will be the lower_snake_case version of the
/// declared variable name.
#[macro_export]
macro_rules! register_convex_counter {
    ($VIS:vis $NAME:ident, $HELP:literal $(,)?) => {
        $VIS static $NAME: std::sync::LazyLock<$crate::prometheus::IntCounter> =
            std::sync::LazyLock::new(|| $crate::register_convex_counter_owned!(
                $NAME,
                $HELP,
            ));
    };
    ($VIS:vis $NAME:ident, $HELP:literal, $LABELS:expr $(,)?) => {
        $VIS static $NAME: std::sync::LazyLock<$crate::prometheus::IntCounterVec> =
            std::sync::LazyLock::new(|| $crate::register_convex_counter_owned!(
                $NAME,
                $HELP,
                $LABELS,
            ));
    };
}

/// Register an integer counter with the Convex metrics registry and return
/// as an expression.
/// An optional third argument allows specifying labels for this metric.
/// The reported metric name will be the lower_snake_case version of the
/// declared variable name.
#[macro_export]
macro_rules! register_convex_counter_owned {
    ($NAME:ident, $HELP:literal $(,)?) => {{
        $crate::paste! {
            let name = $crate::metric_name!(stringify!([<$NAME:lower>]));
        }
        let help = $crate::metric_help!($HELP);
        #[allow(clippy::disallowed_macros)]
        $crate::prometheus::register_int_counter_with_registry!(
            &*name,
            &*help,
            $crate::CONVEX_METRICS_REGISTRY,
        )
        .expect("Metric initialization failed")
    }};
    ($NAME:ident, $HELP:literal, $LABELS:expr $(,)?) => {{
        $crate::paste! {
            let name = $crate::metric_name!(stringify!([<$NAME:lower>]));
        }
        let help = $crate::metric_help!($HELP);
        #[allow(clippy::disallowed_macros)]
        $crate::prometheus::register_int_counter_vec_with_registry!(
            &*name,
            &*help,
            $LABELS,
            $crate::CONVEX_METRICS_REGISTRY,
        )
        .expect("Metric initialization failed")
    }};
}

/// Register a floating-point gauge with the Convex metrics registry and
/// store in a static variable.
/// An optional third argument allows specifying labels for this metric.
/// The reported metric name will be the lower_snake_case version of the
/// declared variable name.
#[macro_export]
macro_rules! register_convex_gauge {
    ($VIS:vis $NAME:ident, $HELP:literal $(,)?) => {
        $VIS static $NAME: std::sync::LazyLock<$crate::prometheus::Gauge> =
            std::sync::LazyLock::new(|| $crate::register_convex_gauge_owned!(
                $NAME,
                $HELP,
            ));
    };
    ($VIS:vis $NAME:ident, $HELP:literal, $LABELS:expr $(,)?) => {
        $VIS static $NAME: std::sync::LazyLock<$crate::prometheus::GaugeVec> =
            std::sync::LazyLock::new(|| $crate::register_convex_gauge_owned!(
                $NAME,
                $HELP,
                $LABELS,
            ));
    };
}

/// Register a floating-point gauge with the Convex metrics registry and return
/// as an expression.
/// An optional third argument allows specifying labels for this metric.
/// The reported metric name will be the lower_snake_case version of the
/// declared variable name.
#[macro_export]
macro_rules! register_convex_gauge_owned {
    ($NAME:ident, $HELP:literal $(,)?) => {{
        $crate::paste! {
            let name = $crate::metric_name!(stringify!([<$NAME:lower>]));
        }
        let help = $crate::metric_help!($HELP);
        #[allow(clippy::disallowed_macros)]
        $crate::prometheus::register_gauge_with_registry!(
            &*name,
            &*help,
            $crate::CONVEX_METRICS_REGISTRY,
        )
        .expect("Metric initialization failed")
    }};
    ($NAME:ident, $HELP:literal, $LABELS:expr $(,)?) => {{
        $crate::paste! {
            let name = $crate::metric_name!(stringify!([<$NAME:lower>]));
        }
        let help = $crate::metric_help!($HELP);
        #[allow(clippy::disallowed_macros)]
        $crate::prometheus::register_gauge_vec_with_registry!(
            &*name,
            &*help,
            $LABELS,
            $crate::CONVEX_METRICS_REGISTRY,
        )
        .expect("Metric initialization failed")
    }};
}
