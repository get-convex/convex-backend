use std::borrow::Cow;

pub type Labels = Vec<MetricLabel>;

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
pub struct MetricLabel {
    pub key: &'static str,
    pub value: Cow<'static, str>,
}

impl MetricLabel {
    pub const STATUS_CANCELED: MetricLabel = MetricLabel {
        key: "status",
        value: Cow::Borrowed("canceled"),
    };
    pub const STATUS_DEVELOPER_ERROR: MetricLabel = MetricLabel {
        key: "status",
        value: Cow::Borrowed("developer_error"),
    };
    pub const STATUS_ERROR: MetricLabel = MetricLabel {
        key: "status",
        value: Cow::Borrowed("error"),
    };
    pub const STATUS_SUCCESS: MetricLabel = MetricLabel {
        key: "status",
        value: Cow::Borrowed("success"),
    };

    pub fn new(key: &'static str, value: impl Into<Cow<'static, str>>) -> Self {
        Self {
            key,
            value: value.into(),
        }
    }

    pub const fn new_const(key: &'static str, value: &'static str) -> Self {
        Self {
            key,
            value: Cow::Borrowed(value),
        }
    }

    /// Common labels. Use these instead of custom defined ones when possible.
    pub fn status(is_ok: bool) -> MetricLabel {
        if is_ok {
            Self::STATUS_SUCCESS
        } else {
            Self::STATUS_ERROR
        }
    }

    pub fn split_key_value(&self) -> (&str, &str) {
        (self.key, &self.value)
    }
}

pub const STATUS_LABEL: [&str; 1] = ["status"];

pub trait IntoLabel {
    fn as_label(&self) -> &'static str;
}

impl IntoLabel for bool {
    fn as_label(&self) -> &'static str {
        if *self {
            "true"
        } else {
            "false"
        }
    }
}
