use std::collections::BTreeMap;

use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::{
    codegen_convex_serialization,
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
    pub name: ComponentName,
    pub args: BTreeMap<Identifier, ComponentArgumentValidator>,
    pub child_components: Vec<ComponentInstantiation>,
    pub exports: BTreeMap<Identifier, ComponentExport>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ComponentInstantiation {
    pub name: ComponentName,
    pub path: ComponentDefinitionPath,
    pub args: BTreeMap<Identifier, ComponentArgument>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentExport {
    Branch(BTreeMap<Identifier, ComponentExport>),
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
struct SerializedComponentDefinitionMetadata {
    path: String,
    name: String,
    inputs: Vec<(String, SerializedComponentArgumentValidator)>,
    child_components: Vec<SerializedComponentInstantiation>,
    exports: SerializedComponentExport,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SerializedComponentArgumentValidator {
    Value { value: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SerializedComponentArgument {
    Value { value: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
struct SerializedComponentInstantiation {
    name: String,
    path: String,
    args: Vec<(String, SerializedComponentArgument)>,
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
            name: String::from(m.name),
            inputs: m
                .args
                .into_iter()
                .map(|(k, v)| anyhow::Ok((String::from(k), v.try_into()?)))
                .try_collect()?,
            child_components: m
                .child_components
                .into_iter()
                .map(TryFrom::try_from)
                .try_collect()?,
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
            name: m.name.parse()?,
            args: m
                .inputs
                .into_iter()
                .map(|(k, v)| anyhow::Ok((k.parse()?, v.try_into()?)))
                .try_collect()?,
            child_components: m
                .child_components
                .into_iter()
                .map(TryFrom::try_from)
                .try_collect()?,
            exports,
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
                .into_iter()
                .map(|(k, v)| anyhow::Ok((String::from(k), v.try_into()?)))
                .try_collect()?,
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
                .into_iter()
                .map(|(k, v)| anyhow::Ok((k.parse()?, v.try_into()?)))
                .try_collect()?,
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
                value: serde_json::to_string(&JsonValue::try_from(v)?)?,
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
            prop::collection::btree_map(any::<Identifier>(), inner, 1..4)
                .prop_map(ComponentExport::Branch)
        })
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ComponentDefinitionPath {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = ComponentDefinitionPath>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        (0..=4, prop::collection::vec(any::<ComponentName>(), 0..=4))
            .prop_map(|(depth, components)| {
                let mut path = String::new();
                for _ in 0..depth {
                    path.push_str("../");
                }
                for component in components {
                    path.push_str(&component);
                    path.push('/');
                }
                path.parse().unwrap()
            })
            .boxed()
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
            prop::collection::btree_map(any::<Identifier>(), any::<ComponentArgument>(), 0..4),
        )
            .prop_map(|(name, path, args)| ComponentInstantiation { name, path, args })
    }
}
