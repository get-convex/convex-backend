use std::{
    borrow::Cow,
    collections::{
        btree_map::Entry,
        BTreeMap,
    },
    fmt,
    sync::LazyLock,
};

use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
pub use errors::{
    INTERNAL_SERVER_ERROR,
    INTERNAL_SERVER_ERROR_MSG,
};
use metrics::{
    log_counter,
    SERVICE_NAME,
};
use pb::common::{
    FrameData as FrameDataProto,
    JsError as JsErrorProto,
    JsFrames as JsFramesProto,
};
use rand::Rng;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sourcemap::SourceMap;
use url::Url;
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    ConvexValue,
};

use crate::metrics::log_errors_reported_total;

// Regex to match emails from https://emailregex.com/
pub static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])"#).unwrap()
});

/// Replacers for PII in errors before reporting to thirdparty services
/// (sentry/datadog)
static PII_REPLACEMENTS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // Regex to match PII where we show the object that doesn't match the
        // validator.
        (Regex::new(r"(?s)Object:.*Validator").unwrap(), "Validator"),
        (EMAIL_REGEX.clone(), "*****@*****.***"),
    ]
});

/// Return Result<(), MainError> from main functions to report returned errors
/// to Sentry.
pub struct MainError(anyhow::Error);
impl<T: Into<anyhow::Error>> From<T> for MainError {
    fn from(e: T) -> Self {
        let mut err: anyhow::Error = e.into();
        report_error_sync(&mut err);
        Self(err)
    }
}

impl std::fmt::Debug for MainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Just print the `Display` of the error rather than `Debug`, as `report_error`
        // above will already print the stack trace when `RUST_BACKTRACE` is set.
        write!(f, "{}", self.0)
    }
}

fn strip_pii(err: &mut anyhow::Error) {
    if let Some(error_metadata) = err.downcast_mut::<ErrorMetadata>() {
        for (regex, replacement) in PII_REPLACEMENTS.iter() {
            match regex.replace_all(&error_metadata.msg, *replacement) {
                Cow::Borrowed(b) if b == error_metadata.msg => (),
                cow => error_metadata.msg = Cow::Owned(cow.into_owned()),
            }
        }
    }

    let s = format!("{err:#}");
    let mut transformed = s.clone();
    for (regex, replacement) in PII_REPLACEMENTS.iter() {
        transformed = regex.replace_all(&transformed, *replacement).to_string();
    }
    if s != transformed {
        // How to get the backtrace properly into the anyhow? This is not what we want,
        // but works.
        let em = err.downcast_mut::<ErrorMetadata>().cloned();
        if let Some(em) = em {
            *err = anyhow::anyhow!(err.backtrace().to_string())
                .context(transformed)
                .context(em);
        } else {
            *err = anyhow::anyhow!(err.backtrace().to_string()).context(transformed);
        }
    }
}

/// Log an error to Sentry.
/// This is the one point where we call into Sentry.
///
/// Other parts of codebase should not use the `sentry_anyhow` crate directly!
pub async fn report_error(err: &mut anyhow::Error) {
    // Trace error before yield - since during shutdown, we won't be back.
    trace_error(err);

    // Yield in case this is during shutdown - at which point, errors being reported
    // explicitly aren't useful. Yielding allows tokio to complete a cancellation.
    tokio::task::yield_now().await;

    report_error_sync_no_tracing(err);
}

/// Use the `pub async fn report_error` above if possible to log an error to
/// sentry. This is a synchronous version for use in sync contexts.
pub fn report_error_sync(err: &mut anyhow::Error) {
    trace_error(err);
    report_error_sync_no_tracing(err);
}

fn trace_error(err: &mut anyhow::Error) {
    strip_pii(err);
    if let Some(label) = err.metric_server_error_label() {
        log_errors_reported_total(label);
    }

    let label = err.metric_status_label_value();
    let err_for_tracing = format!("{err:#}").replace("\n", "\\n");
    let full_msg = format!(
        "Caught {label} error (RUST_BACKTRACE=1 RUST_LOG=info,{}=debug for full trace): \
         {err_for_tracing}",
        module_path!()
    );
    if err.metric_server_error_label().is_some() {
        tracing::error!("{full_msg}");
    } else {
        tracing::warn!("{full_msg}");
    }
    tracing::debug!("{err:?}");
}

fn report_error_sync_no_tracing(err: &mut anyhow::Error) {
    if let Some(e) = err.downcast_mut::<ErrorMetadata>() {
        if let Some(counter) = e.custom_metric() {
            log_counter(counter, 1);
        }
        // Set the source of this error to the service name if it's not already set,
        // denoting that this error has been reported and downstream callers that
        // receive this error need not re-report it.
        match &e.source {
            Some(source) => {
                tracing::debug!("Not reporting above error: already reported by {source}");
                return;
            },
            None => {
                e.source = Some(SERVICE_NAME.clone());
            },
        }
    }

    let Some(sentry_client) = sentry::Hub::current().client() else {
        tracing::error!("Not reporting above error: Sentry is not configured");
        return;
    };
    if let Some((level, prob)) = err.should_report_to_sentry() {
        if let Some(prob) = prob
            && rand::rng().random::<f64>() > prob
        {
            tracing::debug!("Not reporting above error to sentry - due to sampling.");
            return;
        }

        if !sentry_client.is_enabled() {
            tracing::debug!("Not reporting above error: SENTRY_DSN not set.");
            return;
        }

        let mut event = event_from_error(err);
        // N.B.: we don't use `sentry::with_scope` because I think that is
        // non-thread-safe if the Hub itself is shared across threads; but we
        // can just attach data directly onto the event.
        event.level = level;
        event
            .tags
            .insert("short_msg".into(), err.short_msg().to_owned());
        let event_id = sentry::capture_event(event);
        tracing::error!(
            "Reporting above error to sentry with event_id {}",
            event_id.simple()
        );
    } else {
        tracing::debug!("Not reporting above error to sentry.");
    }
}

/// Construct a sentry `Event` from an `anyhow` error chain, while inserting
/// `ErrorMetadata`'s `short_msg` into the appropriate type.
fn event_from_error(err: &anyhow::Error) -> sentry::protocol::Event<'static> {
    let mut event = sentry::integrations::anyhow::event_from_error(err);
    if let Some(em) = err.downcast_ref::<ErrorMetadata>() {
        // hacky: we don't know where in the exception chain this
        // `ErrorMetadata` is; and if ErrorMetadata was added via `.context()`
        // (as opposed to being the root cause), the actual error type (as found
        // by `<dyn std::error::Error>::downcast`) won't be ErrorMetadata itself
        // but will be a private ContextError type.
        //
        // So we'll just find the matching exception by string quality :shrug:
        // This doesn't work if there are *multiple* ErrorMetadatas attached to
        // the error but we should generally try to avoid doing that.
        if let Some(exception) = event.exception.iter_mut().find(|e| {
            e.value.as_deref() == Some(&*em.msg) && (e.ty == "ErrorMetadata" || e.ty == "Error")
        }) {
            // N.B. the existing `exception.ty` is `ErrorMetadata` if it's the
            // root cause or `Error` otherwise.
            exception.ty = em.short_msg.to_string();
        }
    }
    event
}

/// Recapture the stack trace. Use this when an error is being handed off
/// to a different context with a different stack (eg from an async worker
/// to a request). The original error and its cause chain (:# representation)
/// will get logged as part of the new error. The original stacktrace will not
/// be part of the new error.
///
/// See https://docs.rs/anyhow/latest/anyhow/struct.Error.html#display-representations
pub async fn recapture_stacktrace(mut err: anyhow::Error) -> anyhow::Error {
    let new_error = recapture_stacktrace_noreport(&err);
    report_error(&mut err).await; // report original error, mutating it to strip pii
    new_error
}

pub fn recapture_stacktrace_noreport(err: &anyhow::Error) -> anyhow::Error {
    let new_error = anyhow::anyhow!("Orig Error: {err:#}.");
    match err.downcast_ref::<ErrorMetadata>() {
        Some(em) => new_error.context(em.clone()),
        None => new_error,
    }
}

#[derive(Clone, Deserialize, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct FrameData {
    pub type_name: Option<String>,
    pub function_name: Option<String>,
    pub method_name: Option<String>,
    pub file_name: Option<String>,
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
    pub eval_origin: Option<String>,
    pub is_top_level: Option<bool>,
    pub is_eval: bool,
    pub is_native: bool,
    pub is_constructor: bool,
    pub is_async: bool,
    pub is_promise_all: bool,
    pub promise_index: Option<u32>,
}

impl From<FrameData> for FrameDataProto {
    fn from(
        FrameData {
            type_name,
            function_name,
            method_name,
            file_name,
            line_number,
            column_number,
            eval_origin,
            is_top_level,
            is_eval,
            is_native,
            is_constructor,
            is_async,
            is_promise_all,
            promise_index,
        }: FrameData,
    ) -> Self {
        Self {
            type_name,
            function_name,
            method_name,
            file_name,
            line_number,
            column_number,
            eval_origin,
            is_top_level,
            is_eval: Some(is_eval),
            is_native: Some(is_native),
            is_constructor: Some(is_constructor),
            is_async: Some(is_async),
            is_promise_all: Some(is_promise_all),
            promise_index,
        }
    }
}

impl From<FrameData> for sentry::protocol::Frame {
    fn from(frame: FrameData) -> Self {
        let function = match frame.function_name {
            Some(f) => f,
            None => match frame.method_name {
                Some(m) => m,
                None => "<anonymous>".to_string(),
            },
        };

        Self {
            function: Some(function),
            filename: frame.file_name.clone(),
            lineno: frame.line_number.map(|l| l as u64),
            colno: frame.column_number.map(|c| c as u64),
            module: None,
            package: None,
            abs_path: None,
            pre_context: vec![],
            context_line: None,
            post_context: vec![],
            in_app: Some(
                frame
                    .file_name
                    .map(|f| !f.contains("node_modules"))
                    .unwrap_or(false),
            ),
            vars: BTreeMap::new(),
            image_addr: None,
            instruction_addr: None,
            symbol_addr: None,
            addr_mode: None,
            symbol: None,
        }
    }
}

impl TryFrom<FrameDataProto> for FrameData {
    type Error = anyhow::Error;

    fn try_from(
        FrameDataProto {
            type_name,
            function_name,
            method_name,
            file_name,
            line_number,
            column_number,
            eval_origin,
            is_top_level,
            is_eval,
            is_native,
            is_constructor,
            is_async,
            is_promise_all,
            promise_index,
        }: FrameDataProto,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            type_name,
            function_name,
            method_name,
            file_name,
            line_number,
            column_number,
            eval_origin,
            is_top_level,
            is_eval: is_eval.ok_or_else(|| anyhow::anyhow!("Missing is_eval"))?,
            is_native: is_native.ok_or_else(|| anyhow::anyhow!("Missing is_native"))?,
            is_constructor: is_constructor
                .ok_or_else(|| anyhow::anyhow!("Missing is_constructor"))?,
            is_async: is_async.ok_or_else(|| anyhow::anyhow!("Missing is_async"))?,
            is_promise_all: is_promise_all
                .ok_or_else(|| anyhow::anyhow!("Missing is_promise_all"))?,
            promise_index,
        })
    }
}

impl HeapSize for FrameData {
    fn heap_size(&self) -> usize {
        self.type_name.heap_size()
            + self.function_name.heap_size()
            + self.method_name.heap_size()
            + self.file_name.heap_size()
            + self.line_number.heap_size()
            + self.column_number.heap_size()
            + self.eval_origin.heap_size()
            + self.is_top_level.heap_size()
            + self.is_eval.heap_size()
            + self.is_native.heap_size()
            + self.is_constructor.heap_size()
            + self.is_async.heap_size()
            + self.is_promise_all.heap_size()
            + self.promise_index.heap_size()
    }
}

pub type MappedFrame = FrameData;

impl FrameData {
    fn is_omittable_internal_frame(&self) -> bool {
        let Some(ref f) = self.file_name else {
            return false;
        };
        f.contains("udf-runtime/src") || f.contains("convex/src/server/impl")
    }
}

impl fmt::Display for FrameData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "    at ")?;
        if self.is_async {
            write!(f, "async ")?;
        }
        if self.is_promise_all {
            if let Some(promise_index) = self.promise_index {
                write!(f, "Promise.all (index {promise_index})")?;
            }
        }
        let is_method_call = !(self.is_top_level == Some(true) || self.is_constructor);
        if is_method_call {
            if let Some(ref function_name) = self.function_name {
                if let Some(ref type_name) = self.type_name {
                    if function_name.starts_with(type_name) {
                        write!(f, "{type_name}.")?;
                    }
                }
                write!(f, "{function_name}")?;
                if let Some(ref method_name) = self.method_name {
                    if function_name.ends_with(method_name) {
                        write!(f, " [as {method_name}]")?;
                    }
                }
            } else {
                if let Some(ref type_name) = self.type_name {
                    write!(f, "{type_name}.")?;
                }
                if let Some(ref method_name) = self.method_name {
                    write!(f, "{method_name}")?;
                } else {
                    write!(f, "<anonymous>")?;
                }
            }
        } else if self.is_constructor {
            write!(f, "new ")?;
            if let Some(ref function_name) = self.function_name {
                write!(f, "{function_name}")?;
            } else {
                write!(f, "<anonymous>")?;
            }
        } else if let Some(ref function_name) = self.function_name {
            write!(f, "{function_name}")?;
        } else {
            format_location(f, self)?;
            return Ok(());
        }
        write!(f, " (")?;
        format_location(f, self)?;
        write!(f, ")")?;
        Ok(())
    }
}

/// An Error emitted from a Convex Function execution.
#[derive(Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct JsError {
    pub message: String,
    pub custom_data: Option<ConvexValue>,
    pub frames: Option<JsFrames>,
}

impl From<JsError> for anyhow::Error {
    fn from(js_error: JsError) -> anyhow::Error {
        let msg = js_error.to_string();
        anyhow::anyhow!(ErrorMetadata::bad_request("Error", msg)).context(js_error)
    }
}

impl TryFrom<JsError> for JsErrorProto {
    type Error = anyhow::Error;

    fn try_from(
        JsError {
            message,
            custom_data,
            frames,
        }: JsError,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            message: Some(message),
            custom_data: custom_data
                .map(|v| anyhow::Ok(v.json_serialize()?.into_bytes()))
                .transpose()?,
            frames: frames.map(JsFramesProto::from),
        })
    }
}

impl TryFrom<JsErrorProto> for JsError {
    type Error = anyhow::Error;

    fn try_from(
        JsErrorProto {
            message,
            custom_data,
            frames,
        }: JsErrorProto,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            message: message.ok_or_else(|| anyhow::anyhow!("Missing message"))?,
            custom_data: custom_data
                .map(|bytes| {
                    let json: JsonValue = serde_json::from_slice(&bytes)?;
                    let developer_value = json.try_into()?;
                    anyhow::Ok::<ConvexValue>(developer_value)
                })
                .transpose()?,
            frames: frames.map(JsFrames::try_from).transpose()?,
        })
    }
}

impl HeapSize for JsError {
    fn heap_size(&self) -> usize {
        self.message.heap_size() + self.frames.heap_size()
    }
}

#[derive(Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct JsFrames(pub WithHeapSize<Vec<MappedFrame>>);

impl From<JsFrames> for JsFramesProto {
    fn from(JsFrames(frames): JsFrames) -> Self {
        Self {
            frames: frames.into_iter().map(FrameDataProto::from).collect(),
        }
    }
}

impl TryFrom<JsFramesProto> for JsFrames {
    type Error = anyhow::Error;

    fn try_from(JsFramesProto { frames }: JsFramesProto) -> anyhow::Result<Self> {
        Ok(Self(
            frames.into_iter().map(FrameData::try_from).try_collect()?,
        ))
    }
}

impl HeapSize for JsFrames {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl JsError {
    pub fn from_error(e: anyhow::Error) -> Self {
        match e.downcast::<Self>() {
            Ok(js_error) => js_error,
            Err(e) => Self::from_message(e.user_facing_message()),
        }
    }

    pub fn from_error_ref(e: &anyhow::Error) -> Self {
        match e.downcast_ref::<Self>() {
            Some(js_error) => js_error.clone(),
            None => Self::from_message(e.user_facing_message()),
        }
    }

    pub fn from_message(message: String) -> Self {
        Self {
            message,
            custom_data: None,
            frames: None,
        }
    }

    pub fn convex_error(message: String, data: ConvexValue) -> Self {
        Self {
            message,
            custom_data: Some(data),
            frames: None,
        }
    }

    pub fn from_frames(
        message: String,
        frame_data: Vec<FrameData>,
        custom_data: Option<ConvexValue>,
        mut lookup_source_map: impl FnMut(&Url) -> anyhow::Result<Option<SourceMap>>,
    ) -> Self {
        let mut source_maps = BTreeMap::new();
        let mut mapped_frames = Vec::with_capacity(frame_data.len());
        for mut frame in frame_data {
            if let FrameData {
                file_name: Some(ref f),
                line_number: Some(l),
                column_number: Some(c),
                ..
            } = frame
            {
                let Ok(specifier) = Url::parse(f) else {
                    // We expect the file_name to be fully qualified URL but seems
                    // this is not always the case. Lets log warning here.
                    tracing::warn!("Skipping frame with invalid file_name: {f}");
                    continue;
                };
                let source_map = match source_maps.entry(specifier) {
                    Entry::Vacant(e) => {
                        let maybe_source_map = match lookup_source_map(e.key()) {
                            Ok(maybe_source_map) => maybe_source_map,
                            Err(err) => {
                                // This is not expected so report an error.
                                let mut err = err
                                    .context(ErrorMetadata::operational_internal_server_error())
                                    .context("Failed to lookup source_map");
                                report_error_sync(&mut err);
                                continue;
                            },
                        };
                        let Some(source_map) = maybe_source_map else {
                            tracing::debug!("Missing source map for {}", e.key());
                            continue;
                        };
                        e.insert(source_map)
                    },
                    Entry::Occupied(e) => e.into_mut(),
                };
                if let Some(token) = source_map.lookup_token(l, c) {
                    if let Some(mapped_name) = token.get_source() {
                        frame.file_name = Some(mapped_name.to_string());
                    }
                    frame.line_number = Some(token.get_src_line());
                    frame.column_number = Some(token.get_src_col());
                } else {
                    tracing::debug!("Failed to find token for {f}:{l}:{c}");
                }
            } else {
                tracing::debug!("Skipping incomplete frame: {frame:?}");
            }

            // Omit leading frames inside of our own UDF harness code. This stuff is not
            // helpful to Convex developers - they want to see their own code.
            if mapped_frames.is_empty() && frame.is_omittable_internal_frame() {
                continue;
            }
            mapped_frames.push(frame);
        }

        // Omit trailing frames inside our own UDF harness code as well.
        while let Some(f) = mapped_frames.last()
            && f.is_omittable_internal_frame()
        {
            mapped_frames.pop();
        }

        JsError {
            message,
            custom_data,
            frames: Some(JsFrames(mapped_frames.into())),
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn from_frames_for_test(message: &str, frames: Vec<&str>) -> Self {
        let frame_data = frames
            .into_iter()
            .map(|filename| FrameData {
                file_name: Some(filename.to_string()),
                ..Default::default()
            })
            .collect();
        Self::from_frames(message.to_string(), frame_data, None, |_| Ok(None))
    }
}

// Based on deno's `02_error.js:formatLocation`.
fn format_location(f: &mut fmt::Formatter<'_>, frame: &MappedFrame) -> fmt::Result {
    if frame.is_native {
        return write!(f, "native");
    }
    if let Some(ref file_name) = frame.file_name {
        write!(f, "{file_name}")?;
    } else {
        if frame.is_eval {
            if let Some(ref eval_origin) = frame.eval_origin {
                write!(f, "{eval_origin}, ")?;
            }
        }
        write!(f, "<anonymous>")?;
    }
    if let Some(line_number) = frame.line_number {
        write!(f, ":{line_number}")?;
        if let Some(column_number) = frame.column_number {
            write!(f, ":{column_number}")?;
        }
    }
    Ok(())
}

impl fmt::Debug for JsFrames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for frame in self.0.iter() {
            writeln!(f, "{frame}")?;
        }
        Ok(())
    }
}

impl fmt::Display for JsFrames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl fmt::Debug for JsError {
    // Based on deno's `02_error.js:formatCallsite`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.message)?;
        if let Some(ref frames) = self.frames {
            write!(f, "{frames}")?;
        }
        Ok(())
    }
}

impl fmt::Display for JsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Lease Lost")]
pub struct LeaseLostError;
pub fn lease_lost_error() -> anyhow::Error {
    anyhow::anyhow!(LeaseLostError).context(ErrorMetadata::operational_internal_server_error())
}

#[derive(thiserror::Error, Debug)]
#[error("Database Timeout ({0})")]
pub struct DatabaseTimeoutError(&'static str);
pub fn database_timeout_error(db_type: &'static str) -> anyhow::Error {
    anyhow::anyhow!(DatabaseTimeoutError(db_type))
        .context(ErrorMetadata::operational_internal_server_error())
}

pub const AUTH_ERROR: &str = "AuthError";
pub const TIMEOUT_ERROR_MESSAGE: &str = "Your request timed out.";

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use errors::{
        ErrorMetadata,
        ErrorMetadataAnyhowExt,
    };
    use maplit::btreemap;
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::obj;

    use super::{
        strip_pii,
        FrameDataProto,
        JsError,
        JsErrorProto,
    };
    use crate::{
        errors::{
            event_from_error,
            FrameData,
        },
        schemas::{
            validator::{
                ValidationContext,
                ValidationError,
            },
            SchemaEnforcementError,
        },
    };

    #[test]
    fn test_js_error_conversion_into_anyhow() -> anyhow::Result<()> {
        let js_error = JsError::from_message("Big Error".into());
        let err: anyhow::Error = js_error.into();
        assert_eq!(err.to_string(), "Big Error\n");
        assert_eq!(err.downcast_ref::<JsError>().unwrap().message, "Big Error");
        assert_eq!(err.downcast::<ErrorMetadata>().unwrap().short_msg, "Error");
        Ok(())
    }

    #[test]
    fn test_strip_pii_obj() -> anyhow::Result<()> {
        let object = obj!("foo" => "bar")?;
        let validation_error = ValidationError::ExtraField {
            object: object.clone(),
            field_name: "field".parse()?,
            object_validator: crate::schemas::validator::ObjectValidator(btreemap! {}),
            context: ValidationContext::new(),
        };
        let schema_enforcement_error = SchemaEnforcementError::Document {
            validation_error,
            table_name: "table".parse()?,
        };
        let error_metadata: ErrorMetadata = schema_enforcement_error.to_error_metadata();
        let mut anyhow_err: anyhow::Error = error_metadata.into();
        assert!(anyhow_err.is_bad_request());
        let err_string = anyhow_err.to_string();
        assert!(err_string.contains(&object.to_string()));
        assert!(err_string.contains("Object contains extra field"));
        strip_pii(&mut anyhow_err);
        assert!(anyhow_err.is_bad_request());
        let err_string = anyhow_err.to_string();
        assert!(!err_string.contains(&object.to_string()));
        assert!(err_string.contains("Object contains extra field"));
        Ok(())
    }

    #[test]
    fn test_strip_pii_email() -> anyhow::Result<()> {
        let mut e = anyhow::anyhow!(ErrorMetadata::bad_request(
            "DIY",
            "Need DIY advice? Email totally-not-james@convex.dev"
        ));
        strip_pii(&mut e);
        assert_eq!(e.to_string(), "Need DIY advice? Email *****@*****.***");
        Ok(())
    }

    #[test]
    fn test_strip_pii_wrap_error_message() -> anyhow::Result<()> {
        let mut e = anyhow::anyhow!(ErrorMetadata::bad_request(
            "DIY",
            "Need DIY advice? Email totally-not-james@convex.dev"
        ))
        .wrap_error_message(|m| format!("Wrapped: {m}"));

        strip_pii(&mut e);
        assert!(!format!("{e:?}").contains("totally-not-james"));
        assert!(e.is_bad_request());

        Ok(())
    }

    #[test]
    fn test_strip_pii_outside_and_inside_error_metadata() -> anyhow::Result<()> {
        let mut e = anyhow::anyhow!("Contact totally-not-jamwt@convex.dev if we get here").context(
            ErrorMetadata::bad_request(
                "DIY",
                "Need DIY advice? Email totally-not-james@convex.dev",
            ),
        );

        strip_pii(&mut e);
        assert!(!format!("{e:?}").contains("totally-not-james"));
        assert!(!format!("{e:?}").contains("totally-not-jamwt"));
        assert!(e.is_bad_request());

        Ok(())
    }

    #[test]
    fn test_strip_pii_weird_email() -> anyhow::Result<()> {
        let test = "receipts+memo+====@site.com";
        let mut e = anyhow::anyhow!(ErrorMetadata::bad_request(
            "DIY",
            format!("Need DIY advice? Email {test}"),
        ));
        strip_pii(&mut e);
        assert_eq!(e.to_string(), "Need DIY advice? Email *****@*****.***");
        Ok(())
    }

    #[test]
    fn test_strip_pii_without_error_metadata() -> anyhow::Result<()> {
        let test = "receipts+memo+====@site.com";
        let mut e = anyhow::anyhow!("Need DIY advice? Email {test}");
        strip_pii(&mut e);
        assert_eq!(e.to_string(), "Need DIY advice? Email *****@*****.***");
        Ok(())
    }

    #[test]
    fn test_dont_mess_with_non_pii() -> anyhow::Result<()> {
        let mut e = anyhow::anyhow!("Need DIY advice?").context("You're on your own");
        strip_pii(&mut e);
        assert_eq!(format!("{e:#}"), "You're on your own: Need DIY advice?");
        Ok(())
    }

    #[test]
    fn test_js_error_conversion_anyhow_macro() -> anyhow::Result<()> {
        let js_error = JsError::from_message("Big Error".into());
        let err = anyhow::anyhow!(js_error);
        assert_eq!(err.to_string(), "Big Error\n");
        assert_eq!(err.downcast_ref::<JsError>().unwrap().message, "Big Error");
        assert_eq!(err.downcast::<ErrorMetadata>().unwrap().short_msg, "Error");
        Ok(())
    }

    #[test]
    fn test_event_from_error_non_root_cause() {
        let error = anyhow::anyhow!("message").context(ErrorMetadata::bad_request(
            "ShortMsg",
            "user visible message",
        ));
        let event = event_from_error(&error);
        let exceptions: Vec<_> = event
            .exception
            .iter()
            .map(|ex| (ex.ty.as_str(), ex.value.as_deref()))
            .collect();
        assert_eq!(
            exceptions,
            vec![
                ("Error", Some("message")),
                ("ShortMsg", Some("user visible message")),
            ]
        );
    }

    #[test]
    fn test_event_from_error_root_cause() {
        let error = anyhow::anyhow!(ErrorMetadata::bad_request(
            "ShortMsg",
            "user visible message",
        ))
        .context("contextual message");
        let event = event_from_error(&error);
        let exceptions: Vec<_> = event
            .exception
            .iter()
            .map(|ex| (ex.ty.as_str(), ex.value.as_deref()))
            .collect();
        assert_eq!(
            exceptions,
            vec![
                ("ShortMsg", Some("user visible message")),
                ("Error", Some("contextual message")),
            ]
        );
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn js_error_proto_roundtrips(js_error in any::<JsError>()) {
            assert_roundtrips::<JsError, JsErrorProto>(js_error);
        }
        #[test]
        fn frame_data_proto_roundtrips(left in any::<FrameData>()) {
            assert_roundtrips::<FrameData, FrameDataProto>(left);
        }
    }
}
