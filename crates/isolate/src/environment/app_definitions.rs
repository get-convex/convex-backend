use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    str::FromStr,
};

use anyhow::Context;
use common::{
    components::{
        ComponentDefinitionPath,
        ComponentId,
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
use value::{
    base64,
    NamespacedTableMapping,
    TableMappingValue,
    VirtualTableMapping,
};

use super::{
    AsyncOpRequest,
    IsolateEnvironment,
};
use crate::{
    client::EvaluateAppDefinitionsResult,
    concurrency_limiter::ConcurrencyPermit,
    environment::helpers::syscall_error::{
        syscall_description_for_error,
        syscall_name_for_error,
    },
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
    pub dependency_graph: BTreeSet<(Option<ComponentDefinitionPath>, ComponentDefinitionPath)>,
}

impl AppDefinitionEvaluator {
    pub fn new(
        app_definition: ModuleConfig,
        component_definitions: BTreeMap<ComponentDefinitionPath, ModuleConfig>,
        dependency_graph: BTreeSet<(Option<ComponentDefinitionPath>, ComponentDefinitionPath)>,
    ) -> Self {
        Self {
            app_definition,
            component_definitions,
            dependency_graph,
        }
    }

    pub async fn evaluate<RT: Runtime>(
        self,
        client_id: String,
        isolate: &mut Isolate<RT>,
    ) -> anyhow::Result<EvaluateAppDefinitionsResult> {
        let mut visited = BTreeSet::new();
        enum TraversalState {
            FirstVisit(Option<ComponentDefinitionPath>),
            SecondVisit(Option<ComponentDefinitionPath>),
        }

        let mut stack = vec![TraversalState::FirstVisit(None)];
        let mut definitions = BTreeMap::new();

        // Perform a post-order DFS, evaluating dependencies before their parents.
        while let Some(node) = stack.pop() {
            match node {
                TraversalState::FirstVisit(path) => {
                    if !visited.insert(path.clone()) {
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
                        .map(|(_, c)| TraversalState::FirstVisit(Some(c.clone())));
                    stack.extend(dependencies);
                },
                // Evaluating a component definition.
                TraversalState::SecondVisit(Some(path)) => {
                    let component_definition = self
                        .component_definitions
                        .get(&path)
                        .context("Component definition not found")?;
                    let source = FullModuleSource {
                        source: component_definition.source.clone(),
                        source_map: component_definition.source_map.clone(),
                    };
                    let result = self
                        .evaluate_definition(
                            client_id.clone(),
                            isolate,
                            &definitions,
                            COMPONENT_CONFIG_FILE_NAME,
                            source,
                        )
                        .await?;
                    definitions.insert(Some(path), result);
                },
                // Evaluating the app definition.
                TraversalState::SecondVisit(None) => {
                    let source = FullModuleSource {
                        source: self.app_definition.source.clone(),
                        source_map: self.app_definition.source_map.clone(),
                    };
                    let result = self
                        .evaluate_definition(
                            client_id.clone(),
                            isolate,
                            &definitions,
                            APP_CONFIG_FILE_NAME,
                            source,
                        )
                        .await?;
                    definitions.insert(None, result);
                },
            }
        }
        Ok(definitions)
    }

    async fn evaluate_definition<RT: Runtime>(
        &self,
        client_id: String,
        isolate: &mut Isolate<RT>,
        evaluated_components: &BTreeMap<Option<ComponentDefinitionPath>, JsonValue>,
        filename: &str,
        source: FullModuleSource,
    ) -> anyhow::Result<JsonValue> {
        let env = DefinitionEnvironment {
            expected_filename: filename.to_string(),
            source,
            evaluated_definitions: evaluated_components.clone(),
        };

        let (handle, state) = isolate
            .start_request(ComponentId::Root, client_id.into(), env)
            .await?;
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
            serde_v8::from_v8(&mut scope, v8_result)?
        };

        isolate_context.scope.perform_microtask_checkpoint();
        drop(isolate_context);
        handle.take_termination_error()??;

        Ok(result)
    }
}

const COMPONENT_CONFIG_FILE_NAME: &str = "component.config.js";
const APP_CONFIG_FILE_NAME: &str = "app.config.js";

struct DefinitionEnvironment {
    expected_filename: String,
    source: FullModuleSource,

    evaluated_definitions: BTreeMap<Option<ComponentDefinitionPath>, JsonValue>,
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
        _name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "EnvironmentVariablesUnsupported",
            "Environment variables not supported"
        ));
    }

    fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoTableMappingFetchDuringDefinitionEvaluation",
            "Getting the table mapping unsupported when evaluating app definition"
        ))
    }

    fn get_all_table_mappings(
        &mut self,
    ) -> anyhow::Result<(NamespacedTableMapping, VirtualTableMapping)> {
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
                Some(r.map_err(|e| ErrorMetadata::bad_request("InvalidModule", e.to_string()))?);
            let Some(source) = self.evaluated_definitions.get(&def_path) else {
                return Ok(None);
            };
            let synthetic_module = FullModuleSource {
                source: format!(
                    "export default {{ export: () => {{ return {source} }}, \
                     componentDefinitionPath: \"{}\", }}",
                    String::from(def_path.clone().unwrap())
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
