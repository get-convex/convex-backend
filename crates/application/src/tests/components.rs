use std::collections::BTreeMap;

use common::{
    bootstrap_model::components::{
        definition::{
            ComponentArgument,
            ComponentArgumentValidator,
            ComponentDefinitionMetadata,
            ComponentExport,
            ComponentInstantiation,
        },
        ComponentMetadata,
    },
    components::{
        CanonicalizedComponentModulePath,
        ComponentDefinitionId,
        ComponentFunctionPath,
        ComponentId,
        Reference,
        Resource,
    },
    schemas::validator::Validator,
    types::{
        FunctionCaller,
        ModuleEnvironment,
        UdfType,
    },
    RequestId,
};
use database::{
    SystemMetadataModel,
    WriteSource,
    COMPONENTS_TABLE,
    COMPONENT_DEFINITIONS_TABLE,
};
use keybroker::Identity;
use maplit::btreemap;
use model::{
    config::{
        types::{
            ConfigMetadata,
            ModuleConfig,
        },
        ConfigModel,
    },
    modules::{
        function_validators::{
            ArgsValidator,
            ReturnsValidator,
        },
        module_versions::{
            AnalyzedFunction,
            AnalyzedModule,
            Visibility,
        },
    },
    udf_config::types::UdfConfig,
};
use runtime::testing::TestRuntime;
use semver::Version;
use serde_json::json;
use value::ConvexValue;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

// $ COMPONENTS_ENABLED=true cargo test -p application --lib -- --ignored
// component_v8 --nocapture
#[ignore]
#[convex_macro::test_runtime]
async fn test_component_v8_action(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;

    let mut tx = application.begin(Identity::system()).await?;

    let source = r#"
export const bar = async (ctx, args) => {
    if (args.stop) {
        return "hey";
    }
    const argsJson = {
        reference: "_reference/childComponent/chatWaitlist/foo/bar",
        args: { stop: true },
        version: "1.11.3",
        requestId: "",
    };
    const resultStr = await Convex.asyncSyscall(
        "1.0/actions/action",
        JSON.stringify(argsJson),
    );
    const result = JSON.parse(resultStr);
    return "oh " + result;
};
bar.isConvexFunction = true;
bar.isAction = true;
bar.isRegistered = true;
bar.invokeAction = async (requestId, argsStr) => {
  const result = await bar({}, ...JSON.parse(argsStr));
  return JSON.stringify(result);
};
    "#;
    let module = ModuleConfig {
        path: "foo.js".parse()?,
        source: source.to_string(),
        source_map: None,
        environment: ModuleEnvironment::Isolate,
    };
    let mut analyze_results = BTreeMap::new();
    analyze_results.insert(
        CanonicalizedComponentModulePath {
            component: ComponentDefinitionId::Root,
            module_path: "foo.js".parse()?,
        },
        AnalyzedModule {
            functions: vec![AnalyzedFunction {
                name: "bar".parse()?,
                pos: None,
                udf_type: UdfType::Action,
                visibility: Some(Visibility::Public),
                args: ArgsValidator::Unvalidated,
                returns: ReturnsValidator::Unvalidated,
            }]
            .into(),
            http_routes: None,
            cron_specs: None,
            source_mapped: None,
        },
    );
    ConfigModel::new(&mut tx)
        .apply(
            ConfigMetadata::new(),
            vec![module],
            UdfConfig::new_for_test(&rt, Version::new(1, 10, 0)),
            None,
            analyze_results,
            None,
        )
        .await?;

    // Insert metadata for the root.
    let root_component_id = {
        let definition = ComponentDefinitionMetadata {
            path: "".parse()?,
            name: "app".parse()?,
            args: btreemap! {},
            child_components: vec![ComponentInstantiation {
                name: "chatWaitlist".parse()?,
                path: "../node_modules/waitlist".parse()?,
                args: btreemap! {
                    "maxLength".parse()? => ComponentArgument::Value(ConvexValue::Float64(10.)),
                },
            }],
            exports: btreemap! {
                "foo".parse()? => ComponentExport::Branch(btreemap! {
                    "bar".parse()? => ComponentExport::Leaf(Reference::Function("foo:bar".parse()?)),
                })
            },
        };
        let definition_id = SystemMetadataModel::new(&mut tx)
            .insert(&COMPONENT_DEFINITIONS_TABLE, definition.try_into()?)
            .await?;

        let component = ComponentMetadata {
            definition_id: definition_id.internal_id(),
            parent_and_name: None,
            args: btreemap! {},
        };
        let component_id = SystemMetadataModel::new(&mut tx)
            .insert(&COMPONENTS_TABLE, component.try_into()?)
            .await?;
        component_id.internal_id()
    };
    // Insert metadata for the child.
    {
        let definition = ComponentDefinitionMetadata {
            path: "../node_modules/waitlist".parse()?,
            name: "waitlist".parse()?,
            args: btreemap! {
                "maxLength".parse()? => ComponentArgumentValidator::Value(Validator::Float64),
            },
            child_components: vec![],
            exports: btreemap! {
                "foo".parse()? => ComponentExport::Branch(btreemap! {
                    "bar".parse()? => ComponentExport::Leaf(Reference::Function("foo:bar".parse()?)),
                })
            },
        };
        let definition_id = SystemMetadataModel::new(&mut tx)
            .insert(&COMPONENT_DEFINITIONS_TABLE, definition.try_into()?)
            .await?;

        let component = ComponentMetadata {
            definition_id: definition_id.internal_id(),
            parent_and_name: Some((root_component_id, "chatWaitlist".parse()?)),
            args: btreemap! {
                "maxLength".parse()? => Resource::Value(ConvexValue::Float64(10.)),
            },
        };
        SystemMetadataModel::new(&mut tx)
            .insert(&COMPONENTS_TABLE, component.try_into()?)
            .await?;
    }

    application.commit(tx, WriteSource::unknown()).await?;

    let action_return = application
        .action_udf(
            RequestId::new(),
            ComponentFunctionPath {
                component: ComponentId::Root,
                udf_path: "foo:bar".parse()?,
            },
            vec![json!({})],
            Identity::system(),
            FunctionCaller::HttpEndpoint,
        )
        .await??;
    assert_eq!(action_return.value, "oh hey".try_into()?);

    Ok(())
}
