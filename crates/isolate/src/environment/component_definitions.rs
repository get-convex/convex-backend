use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    str::FromStr,
};

use anyhow::Context;
use common::{
    bootstrap_model::components::definition::{
        ComponentDefinitionMetadata,
        ComponentDefinitionType,
        SerializedComponentDefinitionMetadata,
    },
    components::{
        ComponentDefinitionPath,
        ComponentName,
        Resource,
    },
    knobs::{
        DATABASE_UDF_SYSTEM_TIMEOUT,
        DATABASE_UDF_USER_TIMEOUT,
    },
    log_lines::LogLevel,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        EnvVarName,
        EnvVarValue,
    },
};
use deno_core::{
    serde_v8,
    v8::{
        self,
        GetPropertyNamesArgsBuilder,
    },
    ModuleSpecifier,
};
use errors::ErrorMetadata;
use model::{
    config::types::ModuleConfig,
    modules::module_versions::FullModuleSource,
};
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;
use udf::EvaluateAppDefinitionsResult;
use value::{
    base64,
    identifier::Identifier,
    ConvexObject,
    FieldName,
    NamespacedTableMapping,
    TableMappingValue,
};

use super::{
    AsyncOpRequest,
    IsolateEnvironment,
};
use crate::{
    concurrency_limiter::ConcurrencyPermit,
    environment::helpers::syscall_error::{
        syscall_description_for_error,
        syscall_name_for_error,
    },
    helpers,
    isolate::{
        Isolate,
        CONVEX_SCHEME,
    },
    request_scope::RequestScope,
    strings,
    timeout::Timeout,
};

pub struct AppDefinitionEvaluator {
    pub app_definition: ModuleConfig,
    pub component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
    pub dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
    environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
}

impl AppDefinitionEvaluator {
    pub fn new(
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(ComponentDefinitionPath, ComponentDefinitionPath)>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
        system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> Self {
        Self {
            app_definition,
            component_definitions,
            dependency_graph,
            environment_variables,
            system_env_vars,
        }
    }

    pub async fn evaluate<RT: Runtime>(
        self,
        client_id: String,
        isolate: &mut Isolate<RT>,
    ) -> anyhow::Result<EvaluateAppDefinitionsResult> {
        let mut in_progress = BTreeSet::new();
        enum TraversalState {
            FirstVisit(ComponentDefinitionPath),
            SecondVisit(ComponentDefinitionPath),
        }

        let mut stack = vec![TraversalState::FirstVisit(ComponentDefinitionPath::root())];
        let mut definitions = BTreeMap::new();

        // Perform a post-order DFS, evaluating dependencies before their parents.
        while let Some(node) = stack.pop() {
            match node {
                TraversalState::FirstVisit(path) => {
                    if !in_progress.insert(path.clone()) {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "CyclicImport",
                            "Found cyclic definition dependency"
                        ));
                    }
                    stack.push(TraversalState::SecondVisit(path.clone()));
                    let start = (path.clone(), ComponentDefinitionPath::min());
                    let dependencies = self
                        .dependency_graph
                        .range(start..)
                        .take_while(|(p, _)| p == &path)
                        .map(|(_, c)| TraversalState::FirstVisit(c.clone()));
                    stack.extend(dependencies);
                },
                TraversalState::SecondVisit(path) => {
                    let (filename, source) = if path.is_root() {
                        (
                            APP_CONFIG_FILE_NAME,
                            FullModuleSource {
                                source: self.app_definition.source.clone(),
                                source_map: self.app_definition.source_map.clone(),
                            },
                        )
                    } else {
                        let component_definition = self
                            .component_definitions
                            .get(&path)
                            .context("Component definition not found")?;
                        (
                            COMPONENT_CONFIG_FILE_NAME,
                            FullModuleSource {
                                source: component_definition.source.clone(),
                                source_map: component_definition.source_map.clone(),
                            },
                        )
                    };
                    let result = self
                        .evaluate_definition(
                            client_id.clone(),
                            isolate,
                            &path,
                            &definitions,
                            filename,
                            source,
                        )
                        .await?;
                    in_progress.remove(&path);
                    definitions.insert(path, result);
                },
            }
        }
        Ok(definitions)
    }

    async fn evaluate_definition<RT: Runtime>(
        &self,
        client_id: String,
        isolate: &mut Isolate<RT>,
        path: &ComponentDefinitionPath,
        evaluated_components: &BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
        filename: &str,
        source: FullModuleSource,
    ) -> anyhow::Result<ComponentDefinitionMetadata> {
        let environment_variables = if path.is_root() {
            let mut env_vars = self.environment_variables.clone();
            env_vars.extend(self.system_env_vars.clone());
            Some(env_vars)
        } else {
            None
        };
        let env = DefinitionEnvironment {
            expected_filename: filename.to_string(),
            source,
            evaluated_definitions: evaluated_components.clone(),
            environment_variables,
        };

        let (handle, state) = isolate.start_request(client_id.into(), env).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);
        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, false).await?;
        let handle = isolate_context.handle();

        let result = {
            let mut v8_scope = isolate_context.scope();
            let mut scope = RequestScope::<RT, DefinitionEnvironment>::enter(&mut v8_scope);
            let url = ModuleSpecifier::parse(&format!("{CONVEX_SCHEME}:/{filename}"))?;
            let module = scope.eval_module(&url).await?;
            let namespace = module
                .get_module_namespace()
                .to_object(&mut scope)
                .context("Module namespace wasn't an object?")?;
            let default_str = strings::default.create(&mut scope)?;

            if namespace.has(&mut scope, default_str.into()) != Some(true) {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidDefinition",
                    "Definition file is missing a default export"
                ));
            }
            let default_export: v8::Local<v8::Object> = namespace
                .get(&mut scope, default_str.into())
                .context("Failed to get default export")?
                .try_into()
                .map_err(|_| {
                    ErrorMetadata::bad_request(
                        "InvalidDefinition",
                        "Default export is not an object",
                    )
                })?;

            let property_names = namespace
                .get_property_names(&mut scope, GetPropertyNamesArgsBuilder::default().build())
                .context("Failed to get property names")?;
            if property_names.length() != 1 {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidDefinition",
                    "Definition module has more than one export"
                ));
            }

            let export_str = strings::export.create(&mut scope)?;
            if default_export.has(&mut scope, export_str.into()) != Some(true) {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidDefinition",
                    "Default export is missing its export function"
                ));
            }
            let export: v8::Local<v8::Function> = default_export
                .get(&mut scope, export_str.into())
                .context("Failed to get export function")?
                .try_into()
                .map_err(|_| {
                    ErrorMetadata::bad_request(
                        "InvalidDefinition",
                        "Export function is not a function",
                    )
                })?;

            let v8_result = export
                .call(&mut scope, default_export.into(), &[])
                .context("Failed to call export function")?;

            // Inject the component definition path into the exported result.
            let result_obj: v8::Local<v8::Object> = v8_result.try_into().map_err(|_| {
                ErrorMetadata::bad_request("InvalidDefinition", "Export is not an object")
            })?;
            let key = strings::path.create(&mut scope)?;
            let path = String::from(path.clone());
            let value =
                v8::String::new(&mut scope, &path).context("Failed to create string for path")?;
            anyhow::ensure!(result_obj.set(&mut scope, key.into(), value.into()) == Some(true));

            let metadata: SerializedComponentDefinitionMetadata =
                serde_v8::from_v8(&mut scope, v8_result).map_err(|e| {
                    let value = v8::json::stringify(&mut scope, v8_result)
                        .map(|s| s.to_rust_string_lossy(&mut scope))
                        .unwrap_or_else(|| "<unknown>".to_string());
                    ErrorMetadata::bad_request(
                        "InvalidDefinition",
                        format!("Failed to deserialize {value}: {e}"),
                    )
                })?;
            ComponentDefinitionMetadata::try_from(metadata)
                .map_err(|e| ErrorMetadata::bad_request("InvalidDefinition", e.to_string()))?
        };

        isolate_context.scope.perform_microtask_checkpoint();
        drop(isolate_context);
        handle.take_termination_error(None, "evaluate_definition")??;

        Ok(result)
    }
}

pub struct ComponentInitializerEvaluator {
    pub evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
    pub path: ComponentDefinitionPath,
    pub definition: ModuleConfig,
    pub args: BTreeMap<Identifier, Resource>,
    pub name: ComponentName,
}

impl ComponentInitializerEvaluator {
    pub fn new(
        evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
        path: ComponentDefinitionPath,
        definition: ModuleConfig,
        args: BTreeMap<Identifier, Resource>,
        name: ComponentName,
    ) -> Self {
        Self {
            evaluated_definitions,
            path,
            definition,
            args,
            name,
        }
    }

    pub async fn evaluate<RT: Runtime>(
        self,
        client_id: String,
        isolate: &mut Isolate<RT>,
    ) -> anyhow::Result<BTreeMap<Identifier, Resource>> {
        let filename = COMPONENT_CONFIG_FILE_NAME.to_string();
        let env = DefinitionEnvironment {
            expected_filename: filename.clone(),
            source: FullModuleSource {
                source: self.definition.source,
                source_map: self.definition.source_map,
            },
            evaluated_definitions: self.evaluated_definitions,
            environment_variables: None,
        };
        let (handle, state) = isolate.start_request(client_id.into(), env).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);
        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, true).await?;
        let handle = isolate_context.handle();

        let result = {
            let mut v8_scope = isolate_context.scope();
            let mut scope = RequestScope::<RT, DefinitionEnvironment>::enter(&mut v8_scope);
            let url = ModuleSpecifier::parse(&format!("{CONVEX_SCHEME}:/{filename}"))?;
            let module = scope.eval_module(&url).await?;
            let namespace = module
                .get_module_namespace()
                .to_object(&mut scope)
                .context("Module namespace wasn't an object?")?;
            let default_str = strings::default.create(&mut scope)?;

            if namespace.has(&mut scope, default_str.into()) != Some(true) {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidDefinition",
                    "Definition file is missing a default export"
                ));
            }
            let default_export: v8::Local<v8::Object> = namespace
                .get(&mut scope, default_str.into())
                .context("Failed to get default export")?
                .try_into()
                .map_err(|_| {
                    ErrorMetadata::bad_request(
                        "InvalidDefinition",
                        "Default export is not an object",
                    )
                })?;

            let callback_str = strings::_onInitCallbacks.create(&mut scope)?;
            let callbacks: v8::Local<v8::Object> = default_export
                .get(&mut scope, callback_str.into())
                .context("Failed to get _onInitCallbacks")?
                .try_into()
                .map_err(|_| {
                    ErrorMetadata::bad_request(
                        "InvalidDefinition",
                        "_onInitCallbacks is not an object",
                    )
                })?;

            let name_str = v8::String::new(&mut scope, &String::from(self.name))
                .context("Failed to create string for name")?;
            let callback: v8::Local<v8::Function> = callbacks
                .get(&mut scope, name_str.into())
                .context("Failed to get callback")?
                .try_into()
                .map_err(|_| {
                    ErrorMetadata::bad_request("InvalidDefinition", "Callback is not a function")
                })?;

            let mut args_obj = BTreeMap::new();
            for (arg_name, value) in self.args {
                let Resource::Value(value) = value else {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidDefinition",
                        format!("Argument {arg_name} is not a value"),
                    ))
                };
                args_obj.insert(FieldName::from_str(&arg_name)?, value);
            }
            let args_obj = ConvexObject::try_from(args_obj)?;
            let args_json = JsonValue::from(args_obj);
            let args_str = serde_json::to_string(&args_json)?;
            let args_v8_str = v8::String::new(&mut scope, &args_str)
                .context("Failed to create string for args")?;

            let v8_result: v8::Local<v8::String> = callback
                .call(&mut scope, default_export.into(), &[args_v8_str.into()])
                .context("Failed to call callback")?
                .try_into()
                .map_err(|_| {
                    ErrorMetadata::bad_request(
                        "InvalidDefinition",
                        "Callback returned non-string value",
                    )
                })?;
            let result_str = helpers::to_rust_string(&mut scope, &v8_result)?;
            let result_json: JsonValue = serde_json::from_str(&result_str)?;
            let result_obj = ConvexObject::try_from(result_json)?;

            let mut result = BTreeMap::new();
            for (arg_name, value) in BTreeMap::from(result_obj) {
                result.insert(arg_name.parse()?, Resource::Value(value));
            }
            result
        };

        isolate_context.scope.perform_microtask_checkpoint();
        drop(isolate_context);
        handle.take_termination_error(None, "evaluate")??;

        Ok(result)
    }
}

const COMPONENT_CONFIG_FILE_NAME: &str = "convex.config.js";
const APP_CONFIG_FILE_NAME: &str = "convex.config.js";

struct DefinitionEnvironment {
    expected_filename: String,
    source: FullModuleSource,

    evaluated_definitions: BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>,
    /// Environment variables are allowed in app but not in
    /// component config.
    environment_variables: Option<BTreeMap<EnvVarName, EnvVarValue>>,
}

impl<RT: Runtime> IsolateEnvironment<RT> for DefinitionEnvironment {
    fn trace(&mut self, _level: LogLevel, messages: Vec<String>) -> anyhow::Result<()> {
        tracing::warn!(
            "Unexpected Console access when evaluating app definition: {}",
            messages.join(" ")
        );
        Ok(())
    }

    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoRandomDuringDefinitionEvaluation",
            "Math.random unsupported when evaluating app definition"
        ))
    }

    fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoDateDuringDefinitionEvaluation",
            "Date unsupported when evaluating app definition"
        ))
    }

    fn get_environment_variable(
        &mut self,
        name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        self.environment_variables
            .as_ref()
            .map(|env_vars| env_vars.get(&name).cloned())
            .context(ErrorMetadata::bad_request(
                "EnvironmentVariablesUnsupported",
                "Environment variables are only supported in the app's convex.config.ts.",
            ))
    }

    fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoTableMappingFetchDuringDefinitionEvaluation",
            "Getting the table mapping unsupported when evaluating app definition"
        ))
    }

    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoTableMappingFetchDuringDefinitionEvaluation",
            "Getting the table mapping unsupported when evaluating app definition"
        ))
    }

    async fn lookup_source(
        &mut self,
        path: &str,
        _timeout: &mut Timeout<RT>,
        _permit: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<FullModuleSource>> {
        if path == &self.expected_filename {
            return Ok(Some(self.source.clone()));
        }
        if let Some(remainder) = path.strip_prefix("_componentDeps/") {
            let r: anyhow::Result<_> = try {
                let def_path_str = String::from_utf8(base64::decode_urlsafe(remainder)?)?;
                ComponentDefinitionPath::from_str(&def_path_str)?
            };
            let def_path =
                r.map_err(|e| ErrorMetadata::bad_request("InvalidModule", e.to_string()))?;
            let Some(def) = self.evaluated_definitions.get(&def_path) else {
                return Ok(None);
            };
            let serialized_def = SerializedComponentDefinitionMetadata::try_from(def.clone())?;

            let default_name_string = match def.definition_type {
                ComponentDefinitionType::App => anyhow::bail!(ErrorMetadata::bad_request(
                    "NoImportAppDuringDefinitionEvaluation",
                    format!("App should not be imported while evaluating app definition")
                )),
                ComponentDefinitionType::ChildComponent { ref name, args: _ } => name.to_string(),
            };

            let synthetic_module = FullModuleSource {
                source: format!(
                    "export default {{ export: () => {{ return {} }}, componentDefinitionPath: \
                     \"{}\", defaultName: \"{}\"}}",
                    serde_json::to_string(&serialized_def)?,
                    String::from(def_path.clone()),
                    default_name_string
                ),
                source_map: None,
            };
            return Ok(Some(synthetic_module));
        }
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoImportModuleDuringDefinitionEvaluation",
            format!("Can't import {path} while evaluating app definition")
        ))
    }

    fn syscall(&mut self, name: &str, _args: JsonValue) -> anyhow::Result<JsonValue> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoSyscallDuringAppDefinitionEvaluation",
            format!("Syscall {name} unsupported when evaluating app definition")
        ))
    }

    fn start_async_syscall(
        &mut self,
        name: String,
        _args: JsonValue,
        _resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        anyhow::bail!(ErrorMetadata::bad_request(
            format!("No{}DuringAppDefinition", syscall_name_for_error(&name)),
            format!(
                "{} unsupported while evaluating app definition",
                syscall_description_for_error(&name),
            ),
        ))
    }

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        _resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        anyhow::bail!(ErrorMetadata::bad_request(
            format!("No{}DuringAppDefinition", request.name_for_error()),
            format!(
                "{} unsupported while evaluating app definition",
                request.description_for_error()
            ),
        ))
    }

    fn user_timeout(&self) -> std::time::Duration {
        *DATABASE_UDF_USER_TIMEOUT
    }

    fn system_timeout(&self) -> std::time::Duration {
        *DATABASE_UDF_SYSTEM_TIMEOUT
    }
}
