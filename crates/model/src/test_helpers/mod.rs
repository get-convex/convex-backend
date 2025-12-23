use std::collections::BTreeMap;

use async_trait::async_trait;
use common::{
    bootstrap_model::components::{
        definition::{
            ComponentDefinitionMetadata,
            ComponentDefinitionType,
        },
        ComponentMetadata,
        ComponentState,
        ComponentType,
    },
    components::ComponentId,
    runtime::Runtime,
};
use database::{
    test_helpers::{
        DbFixtures,
        DbFixturesArgs,
    },
    BootstrapComponentsModel,
    Database,
    SystemMetadataModel,
    COMPONENTS_TABLE,
    COMPONENT_DEFINITIONS_TABLE,
};
use value::TableNamespace;

use crate::{
    components::config::ComponentConfigModel,
    initialize_application_system_tables,
    virtual_system_mapping,
};

#[async_trait(?Send)]
pub trait DbFixturesWithModel<RT: Runtime>: Sized {
    async fn new_with_model(rt: &RT) -> anyhow::Result<Self>;
    async fn new_with_model_and_args(rt: &RT, args: DbFixturesArgs) -> anyhow::Result<Self>;
}

#[async_trait(?Send)]
impl<RT: Runtime> DbFixturesWithModel<RT> for DbFixtures<RT> {
    async fn new_with_model(rt: &RT) -> anyhow::Result<Self> {
        Self::new_with_model_and_args(
            rt,
            DbFixturesArgs {
                virtual_system_mapping: virtual_system_mapping().clone(),
                ..Default::default()
            },
        )
        .await
    }

    async fn new_with_model_and_args(rt: &RT, args: DbFixturesArgs) -> anyhow::Result<Self> {
        let fixture = Self::new_with_args(rt, args).await?;
        initialize_application_system_tables(&fixture.db).await?;
        Ok(fixture)
    }
}

#[allow(async_fn_in_trait)]
pub trait DatabaseExt {
    /// This creates a namespace and associated dummy component, but it will
    /// have no functions or modules.
    async fn create_namespace_for_test(
        &self,
        parent: TableNamespace,
        name: &str,
    ) -> anyhow::Result<TableNamespace>;
}

impl<RT: Runtime> DatabaseExt for Database<RT> {
    /// This creates a namespace and associated dummy component, but it will
    /// have no functions or modules.
    async fn create_namespace_for_test(
        &self,
        parent: TableNamespace,
        name: &str,
    ) -> anyhow::Result<TableNamespace> {
        let mut tx = self.begin_system().await?;
        let parent_component_id = match parent {
            TableNamespace::Global => {
                if let Some(component) = BootstrapComponentsModel::new(&mut tx).root_component()? {
                    component.id().developer_id
                } else {
                    // Initialize the root component too
                    let root_definition_id = SystemMetadataModel::new_global(&mut tx)
                        .insert(
                            &COMPONENT_DEFINITIONS_TABLE,
                            ComponentDefinitionMetadata {
                                path: "app".parse()?,
                                definition_type: ComponentDefinitionType::App,
                                child_components: Vec::new(),
                                http_mounts: BTreeMap::new(),
                                exports: BTreeMap::new(),
                            }
                            .try_into()?,
                        )
                        .await?;
                    let root_component = SystemMetadataModel::new_global(&mut tx)
                        .insert(
                            &COMPONENTS_TABLE,
                            ComponentMetadata {
                                definition_id: root_definition_id.developer_id,
                                component_type: ComponentType::App,
                                state: ComponentState::Active,
                            }
                            .try_into()?,
                        )
                        .await?;
                    root_component.developer_id
                }
            },
            TableNamespace::ByComponent(id) => id,
        };

        let component_id = ComponentConfigModel::new(&mut tx)
            .initialize_component_namespace(false)
            .await?;
        let definition_id = SystemMetadataModel::new_global(&mut tx)
            .insert(
                &COMPONENT_DEFINITIONS_TABLE,
                ComponentDefinitionMetadata {
                    path: "dummy".parse()?,
                    definition_type: ComponentDefinitionType::ChildComponent {
                        name: name.parse()?,
                        args: BTreeMap::new(),
                    },
                    child_components: Vec::new(),
                    http_mounts: BTreeMap::new(),
                    exports: BTreeMap::new(),
                }
                .try_into()?,
            )
            .await?;
        SystemMetadataModel::new_global(&mut tx)
            .insert_with_internal_id(
                &COMPONENTS_TABLE,
                component_id.internal_id(),
                ComponentMetadata {
                    definition_id: definition_id.developer_id,
                    component_type: ComponentType::ChildComponent {
                        parent: parent_component_id,
                        name: name.parse()?,
                        args: BTreeMap::new(),
                    },
                    state: ComponentState::Active,
                }
                .try_into()?,
            )
            .await?;
        self.commit(tx).await?;
        Ok(ComponentId::Child(component_id).into())
    }
}
