use anyhow::Context;
#[cfg(any(test, feature = "testing"))]
use common::components::ComponentPath;
use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
    },
    errors::JsError,
    identity::InertIdentity,
    log_lines::LogLines,
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        AllowedVisibility,
        UdfType,
    },
    version::{
        Version,
        DEPRECATION_THRESHOLD,
    },
};
use database::{
    unauthorized_error,
    BootstrapComponentsModel,
    Transaction,
};
use errors::ErrorMetadata;
use keybroker::Identity;
use model::{
    backend_state::{
        types::BackendState,
        BackendStateModel,
        DISABLED_ERROR_MESSAGE,
        PAUSED_ERROR_MESSAGE,
    },
    modules::{
        function_validators::ReturnsValidator,
        module_versions::{
            AnalyzedFunction,
            Visibility,
        },
        ModuleModel,
    },
    udf_config::UdfConfigModel,
    virtual_system_mapping,
};
#[cfg(any(test, feature = "testing"))]
use proptest::arbitrary::Arbitrary;
#[cfg(any(test, feature = "testing"))]
use proptest::strategy::Strategy;
use rand::Rng;
use serde_json::Value as JsonValue;
use value::{
    heap_size::HeapSize,
    ConvexArray,
    ConvexValue,
    NamespacedTableMapping,
};

use crate::{
    helpers::validate_udf_args_size,
    parse_udf_args,
    ActionOutcome,
    JsonPackedValue,
    SyscallTrace,
    UdfOutcome,
};
pub async fn validate_schedule_args<RT: Runtime>(
    path: CanonicalizedComponentFunctionPath,
    udf_args: Vec<JsonValue>,
    scheduled_ts: UnixTimestamp,
    udf_ts: UnixTimestamp,
    tx: &mut Transaction<RT>,
) -> anyhow::Result<(CanonicalizedComponentFunctionPath, ConvexArray)> {
    // We validate the following mostly so the developer don't get the timestamp
    // wrong with more than order of magnitude.
    let delta = scheduled_ts.as_secs_f64() - udf_ts.as_secs_f64();
    if delta > 5.0 * 366.0 * 24.0 * 3600.0 {
        anyhow::bail!(ErrorMetadata::bad_request(
            "InvalidScheduledFunctionDelay",
            format!("{scheduled_ts:?} is more than 5 years in the future")
        ));
    }
    if delta < -5.0 * 366.0 * 24.0 * 3600.0 {
        anyhow::bail!(ErrorMetadata::bad_request(
            "InvalidScheduledFunctionDelay",
            format!("{scheduled_ts:?} is more than 5 years in the past")
        ));
    }

    // We do serialize the arguments, so this is likely our fault.
    let udf_args = parse_udf_args(&path.udf_path, udf_args)?;

    // Even though we might use different version of modules when executing,
    // we do validate that the scheduled function exists at time of scheduling.
    // We do it here instead of within transaction in order to leverage the module
    // cache.
    let canonicalized = path.clone();
    let module = ModuleModel::new(tx)
        .get_metadata_for_function(canonicalized.clone())
        .await?
        .with_context(|| {
            let p = String::from(path.udf_path.module().clone());
            let component = if path.component.is_root() {
                "".to_string()
            } else {
                format!("{} ", String::from(path.clone().component))
            };
            ErrorMetadata::bad_request(
                "InvalidScheduledFunction",
                format!("Attempted to schedule function at nonexistent path: {component}{p}",),
            )
        })?;

    // We validate the function name if analyzed modules are available. Note
    // that scheduling was added after we started persisting the result
    // of analyze, we should always validate in practice. We will tighten
    // the interface and make AnalyzedResult non-optional in the future.
    let function_name = canonicalized.udf_path.function_name();
    if let Some(analyze_result) = &module.analyze_result {
        let found = analyze_result
            .functions
            .iter()
            .any(|f| &f.name == function_name);
        if !found {
            anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidScheduledFunction",
                format!(
                    "Attempted to schedule function, but no exported function {} found in the \
                     file: {}{}. Did you forget to export it?",
                    function_name,
                    String::from(path.udf_path.module().clone()),
                    path.component.in_component_str(),
                ),
            ));
        }
    }

    Ok((path, udf_args))
}

fn missing_or_internal_error(path: &CanonicalizedComponentFunctionPath) -> anyhow::Result<String> {
    Ok(format!(
        "Could not find public function for '{}'{}. Did you forget to run `npx convex dev` or \
         `npx convex deploy`?",
        String::from(path.udf_path.clone().strip()),
        path.component.in_component_str()
    ))
}

async fn udf_version<RT: Runtime>(
    path: &CanonicalizedComponentFunctionPath,
    component: ComponentId,
    tx: &mut Transaction<RT>,
) -> anyhow::Result<Result<Version, JsError>> {
    let udf_config = UdfConfigModel::new(tx, component.into()).get().await?;

    let udf_version = match udf_config {
        Some(udf_config) if udf_config.server_version > DEPRECATION_THRESHOLD.npm.unsupported => {
            udf_config.server_version.clone()
        },
        _ => {
            if udf_config.is_none()
                && ModuleModel::new(tx)
                    .get_analyzed_function(path)
                    .await?
                    .is_err()
            {
                // We don't have a UDF config and we can't find the analyzed function.
                // Likely this developer has never pushed before, so give them
                // the missing error message.
                return Ok(Err(JsError::from_message(missing_or_internal_error(path)?)));
            }

            let unsupported = format!(
                "Convex functions at or below version {} are no longer supported. Update your \
                 Convex npm package and then push your functions again with `npx convex deploy` \
                 or `npx convex dev`.",
                DEPRECATION_THRESHOLD.npm.unsupported
            );

            return Ok(Err(JsError::from_message(unsupported)));
        },
    };
    Ok(Ok(udf_version))
}

/// The path and args to a UDF that have already undergone validation.
///
/// This validation includes:
/// - Checking the visibility of the UDF.
/// - Checking that the UDF is the correct type.
/// - Checking the args size.
/// - Checking that the args pass validation.
///
/// This should only be constructed via `ValidatedPathAndArgs::new` to use the
/// type system to enforce that validation is never skipped.
#[derive(Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct ValidatedPathAndArgs {
    path: CanonicalizedComponentFunctionPath,
    args: ConvexArray,
    // Not set for system modules.
    npm_version: Option<Version>,
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for ValidatedPathAndArgs {
    type Parameters = ();

    type Strategy = impl Strategy<Value = ValidatedPathAndArgs>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        any::<(sync_types::CanonicalizedUdfPath, ConvexArray)>().prop_map(|(udf_path, args)| {
            ValidatedPathAndArgs {
                path: CanonicalizedComponentFunctionPath {
                    component: ComponentPath::test_user(),
                    udf_path,
                },
                args,
                npm_version: None,
            }
        })
    }
}

impl ValidatedPathAndArgs {
    /// Check if the function being called matches the allowed visibility and
    /// return a ValidatedPathAndArgs or an appropriate JsError.
    ///
    /// We want to use the same error message for "this function exists, but
    /// with the wrong visibility" and "this function does not exist" so we
    /// don't leak which non-public functions exist.
    pub async fn new<RT: Runtime>(
        allowed_visibility: AllowedVisibility,
        tx: &mut Transaction<RT>,
        path: CanonicalizedComponentFunctionPath,
        args: ConvexArray,
        expected_udf_type: UdfType,
    ) -> anyhow::Result<Result<ValidatedPathAndArgs, JsError>> {
        if path.udf_path.is_system() {
            anyhow::ensure!(
                path.component.is_root(),
                "System module inside non-root component?"
            );

            // We don't analyze system modules, so we don't validate anything
            // except the identity for them.
            let result = if tx.identity().is_admin() || tx.identity().is_system() {
                Ok(ValidatedPathAndArgs {
                    path,
                    args,
                    npm_version: None,
                })
            } else {
                Err(JsError::from_message(
                    unauthorized_error("Executing function").to_string(),
                ))
            };
            return Ok(result);
        }

        let (_, component) = BootstrapComponentsModel::new(tx)
            .component_path_to_ids(path.component.clone())
            .await?;

        let mut backend_state_model = BackendStateModel::new(tx);
        let backend_state = backend_state_model.get_backend_state().await?;
        match backend_state {
            BackendState::Running => {},
            BackendState::Paused => {
                return Ok(Err(JsError::from_message(PAUSED_ERROR_MESSAGE.to_string())));
            },
            BackendState::Disabled => {
                return Ok(Err(JsError::from_message(
                    DISABLED_ERROR_MESSAGE.to_string(),
                )));
            },
        }

        let udf_version = match udf_version(&path, component, tx).await? {
            Ok(udf_version) => udf_version,
            Err(e) => return Ok(Err(e)),
        };

        // AnalyzeResult result should be populated for all supported versions.
        let Ok(analyzed_function) = ModuleModel::new(tx).get_analyzed_function(&path).await? else {
            return Ok(Err(JsError::from_message(missing_or_internal_error(
                &path,
            )?)));
        };

        ValidatedPathAndArgs::new_inner(
            allowed_visibility,
            tx,
            path,
            component,
            args,
            expected_udf_type,
            analyzed_function,
            udf_version,
        )
    }

    /// Do argument validation and get returns validator without retrieving
    /// the analyze result twice.
    pub async fn new_with_returns_validator<RT: Runtime>(
        allowed_visibility: AllowedVisibility,
        tx: &mut Transaction<RT>,
        path: CanonicalizedComponentFunctionPath,
        args: ConvexArray,
        expected_udf_type: UdfType,
    ) -> anyhow::Result<Result<(ValidatedPathAndArgs, ReturnsValidator), JsError>> {
        if path.udf_path.is_system() {
            // We don't analyze system modules, so we don't validate anything
            // except the identity for them.
            let result = if tx.identity().is_admin() || tx.identity().is_system() {
                Ok((
                    ValidatedPathAndArgs {
                        path,
                        args,
                        npm_version: None,
                    },
                    ReturnsValidator::Unvalidated,
                ))
            } else {
                Err(JsError::from_message(
                    unauthorized_error("Executing function").to_string(),
                ))
            };
            return Ok(result);
        }
        let mut backend_state_model = BackendStateModel::new(tx);
        let backend_state = backend_state_model.get_backend_state().await?;
        match backend_state {
            BackendState::Running => {},
            BackendState::Paused => {
                return Ok(Err(JsError::from_message(PAUSED_ERROR_MESSAGE.to_string())));
            },
            BackendState::Disabled => {
                return Ok(Err(JsError::from_message(
                    DISABLED_ERROR_MESSAGE.to_string(),
                )));
            },
        }

        let (_, component) = BootstrapComponentsModel::new(tx)
            .component_path_to_ids(path.component.clone())
            .await?;

        let udf_version = match udf_version(&path, component, tx).await? {
            Ok(udf_version) => udf_version,
            Err(e) => return Ok(Err(e)),
        };

        // AnalyzeResult result should be populated for all supported versions.
        //
        //
        let Ok(analyzed_function) = ModuleModel::new(tx).get_analyzed_function(&path).await? else {
            return Ok(Err(JsError::from_message(missing_or_internal_error(
                &path,
            )?)));
        };

        let returns_validator = if path.udf_path.is_system() {
            ReturnsValidator::Unvalidated
        } else {
            analyzed_function.returns.clone()
        };

        match ValidatedPathAndArgs::new_inner(
            allowed_visibility,
            tx,
            path,
            component,
            args,
            expected_udf_type,
            analyzed_function,
            udf_version,
        )? {
            Ok(validated_udf_path_and_args) => {
                Ok(Ok((validated_udf_path_and_args, returns_validator)))
            },
            Err(js_err) => Ok(Err(js_err)),
        }
    }

    fn new_inner<RT: Runtime>(
        allowed_visibility: AllowedVisibility,
        tx: &mut Transaction<RT>,
        path: CanonicalizedComponentFunctionPath,
        component: ComponentId,
        args: ConvexArray,
        expected_udf_type: UdfType,
        analyzed_function: AnalyzedFunction,
        version: Version,
    ) -> anyhow::Result<Result<ValidatedPathAndArgs, JsError>> {
        let identity = tx.identity();
        match identity {
            // This is an admin, so allow calling all functions
            Identity::InstanceAdmin(_) | Identity::ActingUser(..) => (),
            _ => match allowed_visibility {
                AllowedVisibility::All => (),
                AllowedVisibility::PublicOnly => match analyzed_function.visibility {
                    Some(Visibility::Public) => (),
                    Some(Visibility::Internal) => {
                        return Ok(Err(JsError::from_message(missing_or_internal_error(
                            &path,
                        )?)));
                    },
                    None => {
                        anyhow::bail!(
                            "No visibility found for analyzed function {}{}",
                            path.udf_path,
                            path.component.in_component_str(),
                        );
                    },
                },
            },
        };
        if expected_udf_type != analyzed_function.udf_type {
            anyhow::ensure!(path.component.is_root());
            return Ok(Err(JsError::from_message(format!(
                "Trying to execute {}{} as {}, but it is defined as {}.",
                path.udf_path,
                path.component.in_component_str(),
                expected_udf_type,
                analyzed_function.udf_type
            ))));
        }

        match validate_udf_args_size(&path.udf_path, &args) {
            Ok(()) => (),
            Err(err) => return Ok(Err(err)),
        }

        let table_mapping = &tx.table_mapping().namespace(component.into());

        // If the UDF has an args validator, check that these args match.
        let args_validation_error =
            analyzed_function
                .args
                .check_args(&args, table_mapping, &virtual_system_mapping())?;

        if let Some(error) = args_validation_error {
            return Ok(Err(JsError::from_message(format!(
                "ArgumentValidationError: {error}",
            ))));
        }

        Ok(Ok(ValidatedPathAndArgs {
            path,
            args,
            npm_version: Some(version),
        }))
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_tests(
        path: CanonicalizedComponentFunctionPath,
        args: ConvexArray,
        npm_version: Option<Version>,
    ) -> Self {
        Self {
            path,
            args,
            npm_version,
        }
    }

    pub fn path(&self) -> &CanonicalizedComponentFunctionPath {
        &self.path
    }

    pub fn consume(
        self,
    ) -> (
        CanonicalizedComponentFunctionPath,
        ConvexArray,
        Option<Version>,
    ) {
        (self.path, self.args, self.npm_version)
    }

    pub fn npm_version(&self) -> &Option<Version> {
        &self.npm_version
    }

    pub fn from_proto(
        pb::common::ValidatedPathAndArgs {
            path,
            args,
            npm_version,
            component_path,
        }: pb::common::ValidatedPathAndArgs,
    ) -> anyhow::Result<Self> {
        let args_json: JsonValue =
            serde_json::from_slice(&args.ok_or_else(|| anyhow::anyhow!("Missing args"))?)?;
        let args_value = ConvexValue::try_from(args_json)?;
        let args = ConvexArray::try_from(args_value)?;
        let component = component_path
            .context("Missing component path")?
            .try_into()?;
        Ok(Self {
            path: CanonicalizedComponentFunctionPath {
                component,
                udf_path: path.context("Missing udf_path")?.parse()?,
            },
            args,
            npm_version: npm_version.map(|v| Version::parse(&v)).transpose()?,
        })
    }
}

impl TryFrom<ValidatedPathAndArgs> for pb::common::ValidatedPathAndArgs {
    type Error = anyhow::Error;

    fn try_from(
        ValidatedPathAndArgs {
            path,
            args,
            npm_version,
        }: ValidatedPathAndArgs,
    ) -> anyhow::Result<Self> {
        let args_json = JsonValue::from(args);
        let args = serde_json::to_vec(&args_json)?;
        let component_path = Some(path.component.into());
        Ok(Self {
            path: Some(path.udf_path.to_string()),
            args: Some(args),
            npm_version: npm_version.map(|v| v.to_string()),
            component_path,
        })
    }
}

/// A UDF path that has been validated to be an HTTP route.
///
/// This should only be constructed via `ValidatedHttpRoute::try_from` to use
/// the type system to enforce that validation is never skipped.
pub struct ValidatedHttpPath {
    path: CanonicalizedComponentFunctionPath,
    npm_version: Option<Version>,
}

impl ValidatedHttpPath {
    #[cfg(any(test, feature = "testing"))]
    pub async fn new_for_tests<RT: Runtime>(
        tx: &mut Transaction<RT>,
        udf_path: sync_types::CanonicalizedUdfPath,
        npm_version: Option<Version>,
    ) -> anyhow::Result<Self> {
        if !udf_path.is_system() {
            BackendStateModel::new(tx)
                .fail_while_paused_or_disabled()
                .await?;
        }
        Ok(Self {
            path: CanonicalizedComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path,
            },
            npm_version,
        })
    }

    pub async fn new<RT: Runtime>(
        tx: &mut Transaction<RT>,
        path: CanonicalizedComponentFunctionPath,
    ) -> anyhow::Result<Result<Self, JsError>> {
        // This is not a developer error on purpose.
        anyhow::ensure!(
            path.udf_path.module().as_str() == "http.js",
            "Unexpected http udf path: {:?}",
            path.udf_path,
        );
        if !path.udf_path.is_system() {
            BackendStateModel::new(tx)
                .fail_while_paused_or_disabled()
                .await?;
        }
        let (_, component) = BootstrapComponentsModel::new(tx)
            .component_path_to_ids(path.component.clone())
            .await?;
        let udf_version = match udf_version(&path, component, tx).await? {
            Ok(udf_version) => udf_version,
            Err(e) => return Ok(Err(e)),
        };
        Ok(Ok(ValidatedHttpPath {
            path,
            npm_version: Some(udf_version),
        }))
    }

    pub fn npm_version(&self) -> &Option<Version> {
        &self.npm_version
    }

    pub fn path(&self) -> &CanonicalizedComponentFunctionPath {
        &self.path
    }
}

#[cfg(test)]
mod test {

    use proptest::prelude::*;

    use crate::ValidatedPathAndArgs;
    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_udf_path_proto_roundtrip(v in any::<ValidatedPathAndArgs>()) {
            let proto = pb::common::ValidatedPathAndArgs::try_from(v.clone()).unwrap();
            let v2 = ValidatedPathAndArgs::from_proto(proto).unwrap();
            assert_eq!(v, v2);
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
pub struct ValidatedUdfOutcome {
    pub path: CanonicalizedComponentFunctionPath,
    pub arguments: ConvexArray,
    pub identity: InertIdentity,

    pub rng_seed: [u8; 32],
    pub observed_rng: bool,

    pub unix_timestamp: UnixTimestamp,
    pub observed_time: bool,

    pub log_lines: LogLines,
    pub journal: QueryJournal,

    // QueryUdfOutcomes are stored in the Udf level cache, which is why we would like
    // them to have more compact representation.
    pub result: Result<JsonPackedValue, JsError>,

    pub syscall_trace: SyscallTrace,

    pub udf_server_version: Option<semver::Version>,
}

impl HeapSize for ValidatedUdfOutcome {
    fn heap_size(&self) -> usize {
        self.path.heap_size()
            + self.arguments.heap_size()
            + self.identity.heap_size()
            + self.log_lines.heap_size()
            + self.journal.heap_size()
            + self.result.heap_size()
            + self.syscall_trace.heap_size()
    }
}

impl ValidatedUdfOutcome {
    /// Used for synthesizing an outcome when we encounter an error before
    /// reaching the isolate.
    pub fn from_error(
        js_error: JsError,
        path: CanonicalizedComponentFunctionPath,
        arguments: ConvexArray,
        identity: InertIdentity,
        rt: impl Runtime,
        udf_server_version: Option<semver::Version>,
    ) -> anyhow::Result<Self> {
        Ok(ValidatedUdfOutcome {
            path,
            arguments,
            identity,
            rng_seed: rt.with_rng(|rng| rng.gen()),
            observed_rng: false,
            unix_timestamp: rt.unix_timestamp(),
            observed_time: false,
            log_lines: vec![].into(),
            journal: QueryJournal::new(),
            result: Err(js_error),
            syscall_trace: SyscallTrace::new(),
            udf_server_version,
        })
    }

    pub fn new(
        outcome: UdfOutcome,
        returns_validator: ReturnsValidator,
        table_mapping: &NamespacedTableMapping,
    ) -> Self {
        let mut validated = ValidatedUdfOutcome {
            path: outcome.path,
            arguments: outcome.arguments,
            identity: outcome.identity,
            rng_seed: outcome.rng_seed,
            observed_rng: outcome.observed_rng,
            unix_timestamp: outcome.unix_timestamp,
            observed_time: outcome.observed_time,
            log_lines: outcome.log_lines,
            journal: outcome.journal,
            result: outcome.result,
            syscall_trace: outcome.syscall_trace,
            udf_server_version: outcome.udf_server_version,
        };

        // TODO(CX-6318) Don't pack json value until it's been validated.
        let returns: ConvexValue = match &validated.result {
            Ok(json_packed_value) => json_packed_value.unpack(),
            Err(_) => return validated,
        };

        if let Some(js_err) =
            returns_validator.check_output(&returns, table_mapping, &virtual_system_mapping())
        {
            validated.result = Err(js_err);
        };
        validated
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
pub struct ValidatedActionOutcome {
    pub path: CanonicalizedComponentFunctionPath,
    pub arguments: ConvexArray,
    pub identity: InertIdentity,

    pub unix_timestamp: UnixTimestamp,

    pub result: Result<JsonPackedValue, JsError>,
    pub syscall_trace: SyscallTrace,

    pub udf_server_version: Option<semver::Version>,
}

impl ValidatedActionOutcome {
    pub fn new(
        outcome: ActionOutcome,
        returns_validator: ReturnsValidator,
        table_mapping: &NamespacedTableMapping,
    ) -> Self {
        let mut validated = ValidatedActionOutcome {
            path: outcome.path,
            arguments: outcome.arguments,
            identity: outcome.identity,
            unix_timestamp: outcome.unix_timestamp,
            result: outcome.result,
            syscall_trace: outcome.syscall_trace,
            udf_server_version: outcome.udf_server_version,
        };

        if let Ok(ref json_packed_value) = &validated.result {
            let output = json_packed_value.unpack();
            if let Some(js_err) =
                returns_validator.check_output(&output, table_mapping, &virtual_system_mapping())
            {
                validated.result = Err(js_err);
            }
        }

        validated
    }

    /// Used for synthesizing an outcome when we encounter an error before
    /// reaching the isolate.
    pub fn from_error(
        js_error: JsError,
        path: CanonicalizedComponentFunctionPath,
        arguments: ConvexArray,
        identity: InertIdentity,
        rt: impl Runtime,
        udf_server_version: Option<semver::Version>,
    ) -> Self {
        ValidatedActionOutcome {
            path,
            arguments,
            identity,
            unix_timestamp: rt.unix_timestamp(),
            result: Err(js_error),
            syscall_trace: SyscallTrace::new(),
            udf_server_version,
        }
    }

    pub fn from_system_error(
        path: CanonicalizedComponentFunctionPath,
        arguments: ConvexArray,
        identity: InertIdentity,
        unix_timestamp: UnixTimestamp,
        e: &anyhow::Error,
    ) -> ValidatedActionOutcome {
        ValidatedActionOutcome {
            path,
            arguments,
            identity,
            unix_timestamp,
            result: Err(JsError::from_error_ref(e)),
            syscall_trace: SyscallTrace::new(),
            udf_server_version: None,
        }
    }
}
