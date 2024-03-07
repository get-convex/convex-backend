use std::{
    borrow::Cow,
    ops::Deref,
};

pub type Tags = Vec<MetricTag>;

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug, derive_more::Display)]
pub struct MetricTag(Cow<'static, str>);

impl Deref for MetricTag {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0[..]
    }
}

pub fn metric_tag(tag: impl Into<Cow<'static, str>>) -> MetricTag {
    MetricTag(tag.into())
}
pub const fn metric_tag_const(tag: &'static str) -> MetricTag {
    MetricTag(Cow::Borrowed(tag))
}
pub fn metric_tag_const_value(key: &'static str, value: &'static str) -> MetricTag {
    MetricTag(format!("{}:{}", key, value).into())
}

impl MetricTag {
    pub const STATUS_CANCELED: MetricTag = MetricTag(Cow::Borrowed("status:canceled"));
    pub const STATUS_DEVELOPER_ERROR: MetricTag =
        MetricTag(Cow::Borrowed("status:developer_error"));
    pub const STATUS_ERROR: MetricTag = MetricTag(Cow::Borrowed("status:error"));
    pub const STATUS_SUCCESS: MetricTag = MetricTag(Cow::Borrowed("status:success"));

    /// Common tags. Use these instead of custom defined ones when possible.
    pub fn status(is_ok: bool) -> MetricTag {
        if is_ok {
            Self::STATUS_SUCCESS
        } else {
            Self::STATUS_ERROR
        }
    }

    // TODO: Once we're no longer reporting metrics to Datadog, we should store tag
    // keys and values separately so we don't need to do any splitting/joining.
    pub fn split_key_value(&self) -> (&str, &str) {
        self.0.split_once(':').unwrap_or((&self.0, "true"))
    }
}

pub const STATUS_LABEL: [&str; 1] = ["status"];
