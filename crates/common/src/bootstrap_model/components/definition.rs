use std::{
    collections::BTreeMap,
    ops::Deref,
    str::FromStr,
};

use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use sync_types::path::PathComponent;
use value::{
    codegen_convex_serialization,
    heap_size::HeapSize,
    identifier::Identifier,
    ConvexValue,
};

use crate::{
    components::{
        ComponentDefinitionPath,
        ComponentName,
        Reference,
    },
    schemas::validator::Validator,
};

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ComponentDefinitionMetadata {
    pub path: ComponentDefinitionPath,
    pub definition_type: ComponentDefinitionType,

    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "proptest::collection::vec(proptest::prelude::any::<ComponentInstantiation>(), 0..2)"
        )
    )]
    pub child_components: Vec<ComponentInstantiation>,

    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "proptest::collection::btree_map(proptest::prelude::any::<HttpMountPath>(), \
                             proptest::prelude::any::<Reference>(), 0..2)"
        )
    )]
    pub http_mounts: BTreeMap<HttpMountPath, Reference>,

    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "proptest::collection::btree_map(proptest::prelude::any::<PathComponent>(), \
                        proptest::prelude::any::<ComponentExport>(), 0..4)"
        )
    )]
    pub exports: BTreeMap<PathComponent, ComponentExport>,
}

impl ComponentDefinitionMetadata {
    pub fn default_root() -> Self {
        Self {
            path: ComponentDefinitionPath::root(),
            definition_type: ComponentDefinitionType::App,
            child_components: Vec::new(),
            http_mounts: BTreeMap::new(),
            exports: BTreeMap::new(),
        }
    }

    pub fn is_app(&self) -> bool {
        self.definition_type == ComponentDefinitionType::App
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HttpMountPath(String);

impl Deref for HttpMountPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<HttpMountPath> for String {
    fn from(value: HttpMountPath) -> Self {
        value.0
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for HttpMountPath {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        r"/([a-zA-Z0-9_]/)+".prop_map(|s| s.parse().unwrap())
    }
}

impl HeapSize for HttpMountPath {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl FromStr for HttpMountPath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        anyhow::ensure!(s.starts_with('/'));
        anyhow::ensure!(s.ends_with('/'));
        anyhow::ensure!(!s.contains('*'));
        let path: http::uri::PathAndQuery = s.parse()?;
        anyhow::ensure!(path.query().is_none());
        Ok(Self(s.to_string()))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ComponentDefinitionType {
    App,
    ChildComponent {
        name: ComponentName,
        args: BTreeMap<Identifier, ComponentArgumentValidator>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ComponentInstantiation {
    pub name: ComponentName,
    pub path: ComponentDefinitionPath,
    pub args: Option<BTreeMap<Identifier, ComponentArgument>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentExport {
    Branch(BTreeMap<PathComponent, ComponentExport>),
    Leaf(Reference),
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ComponentArgumentValidator {
    Value(Validator),
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ComponentArgument {
    Value(ConvexValue),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedComponentDefinitionMetadata {
    path: String,
    definition_type: SerializedComponentDefinitionType,
    child_components: Vec<SerializedComponentInstantiation>,
    http_mounts: Option<BTreeMap<String, String>>,
    exports: SerializedComponentExport,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SerializedComponentDefinitionType {
    App {},
    ChildComponent {
        name: String,
        args: Vec<(String, SerializedComponentArgumentValidator)>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SerializedComponentArgumentValidator {
    Value { value: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SerializedComponentArgument {
    Value { value: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
struct SerializedComponentInstantiation {
    name: String,
    path: String,
    args: Option<Vec<(String, SerializedComponentArgument)>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SerializedComponentExport {
    Branch {
        branch: Vec<(String, SerializedComponentExport)>,
    },
    Leaf {
        leaf: String,
    },
}

impl TryFrom<ComponentDefinitionMetadata> for SerializedComponentDefinitionMetadata {
    type Error = anyhow::Error;

    fn try_from(m: ComponentDefinitionMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            path: String::from(m.path),
            definition_type: m.definition_type.try_into()?,
            child_components: m
                .child_components
                .into_iter()
                .map(TryFrom::try_from)
                .try_collect()?,
            http_mounts: Some(
                m.http_mounts
                    .into_iter()
                    .map(|(k, v)| (String::from(k), String::from(v)))
                    .collect(),
            ),
            exports: ComponentExport::Branch(m.exports).try_into()?,
        })
    }
}

impl TryFrom<SerializedComponentDefinitionMetadata> for ComponentDefinitionMetadata {
    type Error = anyhow::Error;

    fn try_from(m: SerializedComponentDefinitionMetadata) -> anyhow::Result<Self> {
        let ComponentExport::Branch(exports) = m.exports.try_into()? else {
            anyhow::bail!("Expected branch of exports at the top level");
        };
        Ok(Self {
            path: m.path.parse()?,
            definition_type: m.definition_type.try_into()?,
            child_components: m
                .child_components
                .into_iter()
                .map(TryFrom::try_from)
                .try_collect()?,
            http_mounts: m
                .http_mounts
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| anyhow::Ok((k.parse()?, v.parse()?)))
                .try_collect()?,
            exports,
        })
    }
}

impl TryFrom<ComponentDefinitionType> for SerializedComponentDefinitionType {
    type Error = anyhow::Error;

    fn try_from(t: ComponentDefinitionType) -> anyhow::Result<Self> {
        Ok(match t {
            ComponentDefinitionType::App => Self::App {},
            ComponentDefinitionType::ChildComponent { name, args } => Self::ChildComponent {
                name: name.to_string(),
                args: args
                    .into_iter()
                    .map(|(k, v)| anyhow::Ok((String::from(k), v.try_into()?)))
                    .try_collect()?,
            },
        })
    }
}

impl TryFrom<SerializedComponentDefinitionType> for ComponentDefinitionType {
    type Error = anyhow::Error;

    fn try_from(t: SerializedComponentDefinitionType) -> anyhow::Result<Self> {
        Ok(match t {
            SerializedComponentDefinitionType::App {} => Self::App,
            SerializedComponentDefinitionType::ChildComponent { name, args } => {
                Self::ChildComponent {
                    name: name.parse()?,
                    args: args
                        .into_iter()
                        .map(|(k, v)| anyhow::Ok((k.parse()?, v.try_into()?)))
                        .try_collect()?,
                }
            },
        })
    }
}

impl TryFrom<ComponentInstantiation> for SerializedComponentInstantiation {
    type Error = anyhow::Error;

    fn try_from(i: ComponentInstantiation) -> anyhow::Result<Self> {
        Ok(Self {
            name: String::from(i.name),
            path: String::from(i.path),
            args: i
                .args
                .map(|args| {
                    args.into_iter()
                        .map(|(k, v)| anyhow::Ok((String::from(k), v.try_into()?)))
                        .try_collect()
                })
                .transpose()?,
        })
    }
}

impl TryFrom<SerializedComponentInstantiation> for ComponentInstantiation {
    type Error = anyhow::Error;

    fn try_from(i: SerializedComponentInstantiation) -> anyhow::Result<Self> {
        Ok(Self {
            name: i.name.parse()?,
            path: i.path.parse()?,
            args: i
                .args
                .map(|args| {
                    args.into_iter()
                        .map(|(k, v)| anyhow::Ok((k.parse()?, v.try_into()?)))
                        .try_collect()
                })
                .transpose()?,
        })
    }
}

impl TryFrom<ComponentArgumentValidator> for SerializedComponentArgumentValidator {
    type Error = anyhow::Error;

    fn try_from(r: ComponentArgumentValidator) -> anyhow::Result<Self> {
        Ok(match r {
            ComponentArgumentValidator::Value(v) => SerializedComponentArgumentValidator::Value {
                value: serde_json::to_string(&JsonValue::try_from(v)?)?,
            },
        })
    }
}

impl TryFrom<SerializedComponentArgumentValidator> for ComponentArgumentValidator {
    type Error = anyhow::Error;

    fn try_from(r: SerializedComponentArgumentValidator) -> anyhow::Result<Self> {
        Ok(match r {
            SerializedComponentArgumentValidator::Value { value: v } => {
                ComponentArgumentValidator::Value(Validator::try_from(serde_json::from_str::<
                    JsonValue,
                >(&v)?)?)
            },
        })
    }
}

impl TryFrom<ComponentArgument> for SerializedComponentArgument {
    type Error = anyhow::Error;

    fn try_from(r: ComponentArgument) -> anyhow::Result<Self> {
        Ok(match r {
            ComponentArgument::Value(v) => SerializedComponentArgument::Value {
                value: serde_json::to_string(&JsonValue::from(v))?,
            },
        })
    }
}

impl TryFrom<SerializedComponentArgument> for ComponentArgument {
    type Error = anyhow::Error;

    fn try_from(r: SerializedComponentArgument) -> anyhow::Result<Self> {
        Ok(match r {
            SerializedComponentArgument::Value { value: v } => ComponentArgument::Value(
                ConvexValue::try_from(serde_json::from_str::<JsonValue>(&v)?)?,
            ),
        })
    }
}

impl TryFrom<ComponentExport> for SerializedComponentExport {
    type Error = anyhow::Error;

    fn try_from(e: ComponentExport) -> anyhow::Result<Self> {
        Ok(match e {
            ComponentExport::Branch(b) => SerializedComponentExport::Branch {
                branch: b
                    .into_iter()
                    .map(|(k, v)| anyhow::Ok((k.to_string(), v.try_into()?)))
                    .try_collect()?,
            },
            ComponentExport::Leaf(f) => SerializedComponentExport::Leaf {
                leaf: String::from(f),
            },
        })
    }
}

impl TryFrom<SerializedComponentExport> for ComponentExport {
    type Error = anyhow::Error;

    fn try_from(e: SerializedComponentExport) -> anyhow::Result<Self> {
        Ok(match e {
            SerializedComponentExport::Branch { branch: b } => ComponentExport::Branch(
                b.into_iter()
                    .map(|(k, v)| anyhow::Ok((k.parse()?, v.try_into()?)))
                    .try_collect()?,
            ),
            SerializedComponentExport::Leaf { leaf: f } => ComponentExport::Leaf(f.parse()?),
        })
    }
}

codegen_convex_serialization!(
    ComponentDefinitionMetadata,
    SerializedComponentDefinitionMetadata
);

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ComponentExport {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = ComponentExport>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        let leaf = any::<Reference>().prop_map(ComponentExport::Leaf);
        leaf.prop_recursive(2, 4, 2, |inner| {
            prop::collection::btree_map(any::<PathComponent>(), inner, 1..4)
                .prop_map(ComponentExport::Branch)
        })
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ComponentInstantiation {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = ComponentInstantiation>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        (
            any::<ComponentName>(),
            any::<ComponentDefinitionPath>(),
            prop::option::of(prop::collection::btree_map(
                any::<Identifier>(),
                any::<ComponentArgument>(),
                0..4,
            )),
        )
            .prop_map(|(name, path, args)| ComponentInstantiation { name, path, args })
    }
}
