use anyhow::Context;
use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
        PublicFunctionPath,
        ResolvedComponentFunctionPath,
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
    backend_state::BackendStateModel,
    components::ComponentsModel,
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
#[cfg(any(test, feature = "testing"))]
use sync_types::CanonicalizedUdfPath;
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

fn missing_or_internal_error(path: PublicFunctionPath) -> anyhow::Result<String> {
    let path = path.debug_into_component_path();
    Ok(format!(
        "Could not find public function for '{}'{}. Did you forget to run `npx convex dev` or \
         `npx convex deploy`?",
        String::from(path.udf_path.clone().strip()),
        path.component.in_component_str()
    ))
}

async fn udf_version<RT: Runtime>(
    path: &ResolvedComponentFunctionPath,
    tx: &mut Transaction<RT>,
) -> anyhow::Result<Result<Version, JsError>> {
    let udf_config = UdfConfigModel::new(tx, path.component.into()).get().await?;

    let udf_version = match udf_config {
        Some(udf_config) if udf_config.server_version > DEPRECATION_THRESHOLD.npm.unsupported => {
            udf_config.server_version.clone()
        },
        _ => {
            if udf_config.is_none()
                && ModuleModel::new(tx)
                    .get_analyzed_function_by_id(path)
                    .await?
                    .is_err()
            {
                // We don't have a UDF config and we can't find the analyzed function.
                // Likely this developer has never pushed before, so give them
                // the missing error message.
                return Ok(Err(JsError::from_message(missing_or_internal_error(
                    PublicFunctionPath::ResolvedComponent(path.clone()),
                )?)));
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
    path: ResolvedComponentFunctionPath,
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

        any::<(
            sync_types::CanonicalizedUdfPath,
            ConvexArray,
            ComponentId,
            ComponentPath,
        )>()
        .prop_map(|(udf_path, args, component_id, component_path)| {
            ValidatedPathAndArgs {
                path: ResolvedComponentFunctionPath {
                    component: component_id,
                    udf_path,
                    component_path: Some(component_path),
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
        path: PublicFunctionPath,
        args: ConvexArray,
        expected_udf_type: UdfType,
    ) -> anyhow::Result<Result<ValidatedPathAndArgs, JsError>> {
        Self::new_with_returns_validator(allowed_visibility, tx, path, args, expected_udf_type)
            .await
            .map(|r| r.map(|(path_and_args, _)| path_and_args))
    }

    /// Do argument validation and get returns validator without retrieving
    /// the analyze result twice.

    #[minitrace::trace]
    pub async fn new_with_returns_validator<RT: Runtime>(
        allowed_visibility: AllowedVisibility,
        tx: &mut Transaction<RT>,
        public_path: PublicFunctionPath,
        args: ConvexArray,
        expected_udf_type: UdfType,
    ) -> anyhow::Result<Result<(ValidatedPathAndArgs, ReturnsValidator), JsError>> {
        if public_path.is_system() {
            let path = match public_path {
                PublicFunctionPath::RootExport(path) => ResolvedComponentFunctionPath {
                    component: ComponentId::Root,
                    udf_path: path.into(),
                    component_path: Some(ComponentPath::root()),
                },
                PublicFunctionPath::Component(path) => {
                    let (_, component) = BootstrapComponentsModel::new(tx)
                        .must_component_path_to_ids(&path.component)?;
                    ResolvedComponentFunctionPath {
                        component,
                        udf_path: path.udf_path,
                        component_path: Some(path.component),
                    }
                },
                PublicFunctionPath::ResolvedComponent(path) => path,
            };
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

        match BackendStateModel::new(tx).fail_while_not_running().await {
            Ok(Ok(())) => {},
            Ok(Err(e)) => {
                return Ok(Err(e));
            },
            Err(e) => return Err(e),
        }

        let path = match public_path.clone() {
            PublicFunctionPath::RootExport(path) => {
                let path = ComponentsModel::new(tx)
                    .resolve_public_export_path(path)
                    .await?;
                let (_, component) = BootstrapComponentsModel::new(tx)
                    .must_component_path_to_ids(&path.component)?;
                ResolvedComponentFunctionPath {
                    component,
                    udf_path: path.udf_path,
                    component_path: Some(path.component),
                }
            },
            PublicFunctionPath::Component(path) => {
                let (_, component) = BootstrapComponentsModel::new(tx)
                    .must_component_path_to_ids(&path.component)?;
                ResolvedComponentFunctionPath {
                    component,
                    udf_path: path.udf_path,
                    component_path: Some(path.component),
                }
            },
            PublicFunctionPath::ResolvedComponent(path) => path,
        };

        let udf_version = match udf_version(&path, tx).await? {
            Ok(udf_version) => udf_version,
            Err(e) => return Ok(Err(e)),
        };

        // AnalyzeResult result should be populated for all supported versions.
        //
        //
        let Ok(analyzed_function) = ModuleModel::new(tx)
            .get_analyzed_function_by_id(&path)
            .await?
        else {
            return Ok(Err(JsError::from_message(missing_or_internal_error(
                public_path,
            )?)));
        };

        let returns_validator = if path.udf_path.is_system() {
            ReturnsValidator::Unvalidated
        } else {
            analyzed_function.returns()?
        };

        match ValidatedPathAndArgs::new_inner(
            allowed_visibility,
            tx,
            path,
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
        path: ResolvedComponentFunctionPath,
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
                            PublicFunctionPath::ResolvedComponent(path),
                        )?)));
                    },
                    None => {
                        anyhow::bail!(
                            "No visibility found for analyzed function {}{}",
                            path.udf_path,
                            path.clone().for_logging().component.in_component_str(),
                        );
                    },
                },
            },
        };
        if expected_udf_type != analyzed_function.udf_type {
            return Ok(Err(JsError::from_message(format!(
                "Trying to execute {}{} as {}, but it is defined as {}.",
                path.udf_path,
                path.clone().for_logging().component.in_component_str(),
                expected_udf_type,
                analyzed_function.udf_type
            ))));
        }

        match validate_udf_args_size(&path.udf_path, &args) {
            Ok(()) => (),
            Err(err) => return Ok(Err(err)),
        }

        let table_mapping = &tx.table_mapping().namespace(path.component.into());

        // If the UDF has an args validator, check that these args match.
        let args_validation_error = analyzed_function.args()?.check_args(
            &args,
            table_mapping,
            &virtual_system_mapping(),
        )?;

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
        udf_path: CanonicalizedUdfPath,
        args: ConvexArray,
        npm_version: Option<Version>,
    ) -> Self {
        Self::new_for_tests_in_component(
            CanonicalizedComponentFunctionPath {
                component: ComponentPath::test_user(),
                udf_path,
            },
            args,
            npm_version,
        )
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_tests_in_component(
        path: CanonicalizedComponentFunctionPath,
        args: ConvexArray,
        npm_version: Option<Version>,
    ) -> Self {
        Self {
            path: ResolvedComponentFunctionPath {
                component: ComponentId::test_user(),
                udf_path: path.udf_path,
                component_path: Some(path.component),
            },
            args,
            npm_version,
        }
    }

    pub fn path(&self) -> &ResolvedComponentFunctionPath {
        &self.path
    }

    pub fn consume(self) -> (ResolvedComponentFunctionPath, ConvexArray, Option<Version>) {
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
            component_id,
        }: pb::common::ValidatedPathAndArgs,
    ) -> anyhow::Result<Self> {
        let args_json: JsonValue =
            serde_json::from_slice(&args.ok_or_else(|| anyhow::anyhow!("Missing args"))?)?;
        let args_value = ConvexValue::try_from(args_json)?;
        let args = ConvexArray::try_from(args_value)?;
        let component = ComponentId::deserialize_from_string(component_id.as_deref())?;
        let component_path = component_path
            .context("Missing component path")?
            .try_into()?;
        Ok(Self {
            path: ResolvedComponentFunctionPath {
                component,
                udf_path: path.context("Missing udf_path")?.parse()?,
                component_path: Some(component_path),
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
        let component_path = path
            .component_path
            .map(|component_path| component_path.into());
        Ok(Self {
            path: Some(path.udf_path.to_string()),
            args: Some(args),
            npm_version: npm_version.map(|v| v.to_string()),
            component_path,
            component_id: path.component.serialize_to_string(),
        })
    }
}

/// A UDF path that has been validated to be an HTTP route.
///
/// This should only be constructed via `ValidatedHttpRoute::try_from` to use
/// the type system to enforce that validation is never skipped.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ValidatedHttpPath {
    path: ResolvedComponentFunctionPath,
    npm_version: Option<Version>,
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for ValidatedHttpPath {
    type Parameters = ();

    type Strategy = impl Strategy<Value = ValidatedHttpPath>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        any::<(sync_types::CanonicalizedUdfPath, ComponentId, ComponentPath)>().prop_map(
            |(udf_path, component_id, component_path)| ValidatedHttpPath {
                path: ResolvedComponentFunctionPath {
                    component: component_id,
                    udf_path,
                    component_path: Some(component_path),
                },
                npm_version: None,
            },
        )
    }
}

impl ValidatedHttpPath {
    #[cfg(any(test, feature = "testing"))]
    pub async fn new_for_tests<RT: Runtime>(
        tx: &mut Transaction<RT>,
        udf_path: sync_types::CanonicalizedUdfPath,
        npm_version: Option<Version>,
    ) -> anyhow::Result<Result<Self, JsError>> {
        if !udf_path.is_system() {
            match BackendStateModel::new(tx).fail_while_not_running().await {
                Ok(Ok(())) => {},
                Ok(Err(e)) => {
                    return Ok(Err(e));
                },
                Err(e) => return Err(e),
            }
        }
        Ok(Ok(Self {
            path: ResolvedComponentFunctionPath {
                component: ComponentId::test_user(),
                udf_path,
                component_path: Some(ComponentPath::test_user()),
            },
            npm_version,
        }))
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
            match BackendStateModel::new(tx).fail_while_not_running().await {
                Ok(Ok(())) => {},
                Ok(Err(e)) => {
                    return Ok(Err(e));
                },
                Err(e) => return Err(e),
            }
        }
        let (_, component) =
            BootstrapComponentsModel::new(tx).must_component_path_to_ids(&path.component)?;
        let path = ResolvedComponentFunctionPath {
            component,
            udf_path: path.udf_path,
            component_path: Some(path.component),
        };
        let udf_version = match udf_version(&path, tx).await? {
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

    pub fn path(&self) -> &ResolvedComponentFunctionPath {
        &self.path
    }

    pub fn from_proto(
        pb::common::ValidatedHttpPath {
            path,
            component_path,
            component_id,
            npm_version,
        }: pb::common::ValidatedHttpPath,
    ) -> anyhow::Result<Self> {
        let component = ComponentId::deserialize_from_string(component_id.as_deref())?;
        let component_path = component_path
            .context("Missing component path")?
            .try_into()?;
        Ok(Self {
            path: ResolvedComponentFunctionPath {
                component,
                udf_path: path.context("Missing udf_path")?.parse()?,
                component_path: Some(component_path),
            },
            npm_version: npm_version.map(|v| Version::parse(&v)).transpose()?,
        })
    }
}

impl TryFrom<ValidatedHttpPath> for pb::common::ValidatedHttpPath {
    type Error = anyhow::Error;

    fn try_from(
        ValidatedHttpPath { path, npm_version }: ValidatedHttpPath,
    ) -> anyhow::Result<Self> {
        let component_path = path
            .component_path
            .map(|component_path| component_path.into());
        Ok(Self {
            path: Some(path.udf_path.to_string()),
            npm_version: npm_version.map(|v| v.to_string()),
            component_path,
            component_id: path.component.serialize_to_string(),
        })
    }
}

#[cfg(test)]
mod test {

    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use crate::{
        ValidatedHttpPath,
        ValidatedPathAndArgs,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_http_action_path_proto_roundtrip(v in any::<ValidatedHttpPath>()) {
            let proto = pb::common::ValidatedHttpPath::try_from(v.clone()).unwrap();
            let v2 = ValidatedHttpPath::from_proto(proto).unwrap();
            assert_eq!(v, v2);
        }

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
        self.path.udf_path.heap_size()
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
            rng_seed: rt.rng().gen(),
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
