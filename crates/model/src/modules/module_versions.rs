use std::{
    collections::BTreeMap,
    convert::{
        TryFrom,
        TryInto,
    },
    mem,
    ops::Deref,
    str::FromStr,
};

use async_lru::async_lru::SizedValue;
use common::types::{
    HttpActionRoute,
    ModuleEnvironment,
    UdfType,
};
use errors::ErrorMetadata;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::Value as JsonValue;
use sync_types::{
    identifier::check_valid_identifier,
    CanonicalizedModulePath,
};
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    id_v6::DocumentIdV6,
    obj,
    ConvexObject,
    ConvexValue,
};

use super::args_validator::ArgsValidator;
use crate::{
    cron_jobs::types::{
        CronIdentifier,
        CronSpec,
    },
    source_packages::types::SourcePackageId,
};

/// System-assigned version number for modules.
pub type ModuleVersion = i64;

/// User-specified JavaScript source code for a module.
pub type ModuleSource = String;

/// Bundler-generated source map for a `ModuleSource`.
pub type SourceMap = String;

/// In-memory representation of a specific version of a module.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ModuleVersionMetadata {
    /// Metadata document for the module we're versioning.
    pub module_id: DocumentIdV6,

    /// Immutable source code for a module version.
    pub source: ModuleSource,
    pub source_package_id: Option<SourcePackageId>,

    // Source map for `source` field above.
    pub source_map: Option<SourceMap>,

    /// Version number for this module version.
    pub version: ModuleVersion,

    // Which environment this module was bundled for.
    pub environment: ModuleEnvironment,

    // Cached result of analyzing this module.
    pub analyze_result: Option<AnalyzedModule>,
}

// A cache size implementation for module cache.
// Implementing this trait here is a hack to get around not being able to
// implement traits for external structs, specifically in the module cache. A
// wrapper struct is an alternative, but it requires changing all callers
// because callers require an Arc value. We could also internalize this
// implementation into the cache but it adds more onerous generics to the
// cache's already long list of types.
impl SizedValue for ModuleVersionMetadata {
    fn size(&self) -> u64 {
        self.heap_size() as u64
    }
}

impl HeapSize for ModuleVersionMetadata {
    fn heap_size(&self) -> usize {
        self.module_id.heap_size()
            + self.source.heap_size()
            + self.source_package_id.heap_size()
            + self.source_map.heap_size()
            + self.version.heap_size()
            + self.analyze_result.heap_size()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct AnalyzedModule {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "value::heap_size::of(prop::collection::vec(any::<AnalyzedFunction>(), \
                        0..4))"
        )
    )]
    pub functions: WithHeapSize<Vec<AnalyzedFunction>>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "prop::option::of(value::heap_size::of(prop::collection::vec(any::<AnalyzedHttpRoute>(), 0..4)))"
        )
    )]
    pub http_routes: Option<WithHeapSize<Vec<AnalyzedHttpRoute>>>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "prop::option::of(value::heap_size::of(prop::collection::btree_map(any::<CronIdentifier>(), \
                        any::<CronSpec>(), 0..4)))"
        )
    )]
    pub cron_specs: Option<WithHeapSize<BTreeMap<CronIdentifier, CronSpec>>>,
    pub source_mapped: Option<MappedModule>,
}

impl HeapSize for AnalyzedModule {
    fn heap_size(&self) -> usize {
        self.functions.heap_size()
            + self.http_routes.heap_size()
            + self.cron_specs.heap_size()
            + self.source_mapped.heap_size()
    }
}

impl TryFrom<AnalyzedModule> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: AnalyzedModule) -> Result<Self, Self::Error> {
        obj!(
            "functions" => value.functions.into_iter().map(ConvexValue::try_from).try_collect::<Vec<_>>()?,
            "sourceMapped" => value.source_mapped.map(ConvexValue::try_from).transpose()?.unwrap_or(ConvexValue::Null),
            "httpRoutes" => match value.http_routes {
                None => ConvexValue::Null,
                Some(http_routes) => http_routes
                    .into_iter()
                    .map(ConvexValue::try_from)
                    .try_collect::<Vec<_>>()?
                    .try_into()?,
            },
            "cronSpecs" => match value.cron_specs {
                None => ConvexValue::Null,
                Some(specs) => {
                    // Array of objects with { identifier: string, spec: CronSpec }
                    let mut arr: Vec<ConvexValue> = vec![];
                    for (identifier, cron_spec) in specs {
                        let spec_object = ConvexObject::try_from(cron_spec)?;
                        let identifier = ConvexValue::try_from(identifier.to_string())?;
                        let obj = obj!(
                            "identifier" => identifier,
                            "spec" => spec_object,
                        )?;
                        arr.push(ConvexValue::from(obj));
                    }
                    ConvexValue::try_from(arr)?
                }
            }
        )
    }
}

impl TryFrom<AnalyzedModule> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: AnalyzedModule) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

impl TryFrom<ConvexObject> for AnalyzedModule {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);
        let functions = match fields.remove("functions") {
            Some(ConvexValue::Array(s)) => s
                .into_iter()
                .map(|v| AnalyzedFunction::try_from(ConvexObject::try_from(v)?))
                .collect::<anyhow::Result<WithHeapSize<Vec<_>>>>()?,
            v => anyhow::bail!("Invalid name field for AnalyzedModule: {v:?}"),
        };
        let source_mapped = match fields.remove("sourceMapped") {
            Some(ConvexValue::Object(o)) => Some(o.try_into()?),
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid sourceMapped field for AnalyzedModule: {v:?}"),
        };
        let http_routes = match fields.remove("httpRoutes") {
            Some(ConvexValue::Array(v)) => {
                let mut routes: WithHeapSize<Vec<AnalyzedHttpRoute>> = WithHeapSize::default();
                for item in v.into_iter() {
                    let obj = ConvexObject::try_from(item)?;
                    let route: anyhow::Result<_> = AnalyzedHttpRoute::try_from(obj);
                    routes.push(route?);
                }
                Some(routes)
            },
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid httpRoutes field for AnalyzedModule: {v:?}"),
        };
        let cron_specs = match fields.remove("cronSpecs") {
            Some(ConvexValue::Array(arr)) => {
                let mut specs: WithHeapSize<BTreeMap<CronIdentifier, CronSpec>> =
                    WithHeapSize::default();
                for item in arr {
                    let obj = ConvexObject::try_from(item)?;
                    let mut fields = BTreeMap::from(obj);
                    let identifier: CronIdentifier = match fields.remove("identifier") {
                        Some(ConvexValue::String(s)) => s.parse()?,
                        _ => anyhow::bail!("Invalid identifier field for cronSpecs"),
                    };
                    let spec: CronSpec = match fields.remove("spec") {
                        Some(ConvexValue::Object(o)) => CronSpec::try_from(o)?,
                        _ => anyhow::bail!("Invalid spec field for cronSpecs"),
                    };
                    specs.insert(identifier, spec);
                }
                Some(specs)
            },
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid cronSpecs field for AnalyzedModule: {v:?}"),
        };
        Ok(Self {
            functions,
            http_routes,
            cron_specs,
            source_mapped,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum Visibility {
    Public,
    Internal,
}
impl TryFrom<ConvexObject> for Visibility {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);
        let visibility = match fields.remove("kind") {
            Some(ConvexValue::String(s)) => match String::from(s).as_str() {
                "public" => Visibility::Public,
                "internal" => Visibility::Internal,
                v => anyhow::bail!("Invalid kind field for Visibility: {v:?}"),
            },
            v => anyhow::bail!("Invalid kind field for Visibility: {v:?}"),
        };

        Ok(visibility)
    }
}

impl TryFrom<Visibility> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: Visibility) -> Result<Self, Self::Error> {
        match value {
            Visibility::Public => obj!("kind" => "public"),
            Visibility::Internal => obj!("kind" => "internal"),
        }
    }
}

impl TryFrom<Visibility> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: Visibility) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct AnalyzedSourcePosition {
    pub path: CanonicalizedModulePath,
    pub start_lineno: u32,
    pub start_col: u32,
    // Consider adding end_* in the future
}

impl HeapSize for AnalyzedSourcePosition {
    fn heap_size(&self) -> usize {
        self.path.as_str().heap_size() + self.start_col.heap_size() + self.start_lineno.heap_size()
    }
}

impl TryFrom<AnalyzedSourcePosition> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: AnalyzedSourcePosition) -> Result<Self, Self::Error> {
        obj!(
            "path" => value.path.as_str(),
            "start_lineno" => (value.start_lineno as i64),
            "start_col" => (value.start_col as i64),
        )
    }
}

impl TryFrom<AnalyzedSourcePosition> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: AnalyzedSourcePosition) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

impl TryFrom<ConvexObject> for AnalyzedSourcePosition {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);
        let path = match fields.remove("path") {
            Some(ConvexValue::String(s)) => s.parse()?,
            v => anyhow::bail!("Invalid path for AnalyzedSourcePosition: {v:?}"),
        };
        let start_lineno = match fields.remove("start_lineno") {
            Some(ConvexValue::Int64(i)) => u32::try_from(i)?,
            v => anyhow::bail!("Invalid start_lineno for AnalyzedSourcePosition: {v:?}"),
        };
        let start_col = match fields.remove("start_col") {
            Some(ConvexValue::Int64(i)) => u32::try_from(i)?,
            v => anyhow::bail!("Invalid start_col for AnalyzedSourcePosition: {v:?}"),
        };

        Ok(Self {
            path,
            start_lineno,
            start_col,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub struct FunctionName(String);

impl FromStr for FunctionName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        check_valid_identifier(s)?;
        Ok(FunctionName(s.to_string()))
    }
}

impl Deref for FunctionName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0[..]
    }
}

impl AsRef<str> for FunctionName {
    fn as_ref(&self) -> &str {
        &self.0[..]
    }
}

impl From<FunctionName> for String {
    fn from(function_name: FunctionName) -> Self {
        function_name.0
    }
}

impl HeapSize for FunctionName {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl FunctionName {
    pub fn from_untrusted(s: &str) -> anyhow::Result<Self> {
        match check_valid_identifier(s) {
            Ok(_) => Ok(Self(s.to_string())),
            Err(e) => Err(anyhow::anyhow!(ErrorMetadata::bad_request(
                "InvalidFunctionName",
                format!("Invalid function name: {}", e),
            ))),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for FunctionName {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = FunctionName>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use common::identifier::arbitrary_regexes::IDENTIFIER_REGEX;
        use proptest::prelude::*;
        IDENTIFIER_REGEX.prop_filter_map("Invalid IdentifierFieldName", |s| {
            FunctionName::from_str(&s).ok()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct AnalyzedFunction {
    pub name: FunctionName,
    pub pos: Option<AnalyzedSourcePosition>,
    pub udf_type: UdfType,
    pub visibility: Option<Visibility>,
    pub args: ArgsValidator,
}

impl HeapSize for AnalyzedFunction {
    fn heap_size(&self) -> usize {
        // Undercount ArgsValidator for simplicity sake.
        self.name.heap_size()
            + mem::size_of::<UdfType>()
            + mem::size_of::<Visibility>()
            + mem::size_of::<ArgsValidator>()
    }
}

impl TryFrom<AnalyzedFunction> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: AnalyzedFunction) -> Result<Self, Self::Error> {
        let args_json = JsonValue::try_from(value.args)?;

        obj!(
            "name" => String::from(value.name),
            "pos" => match value.pos {
                None => ConvexValue::Null,
                Some(pos) => ConvexValue::try_from(pos)?,
            },
            "udfType" => value.udf_type.to_string(),
            "visibility" => match value.visibility {
                None => ConvexValue::Null,
                Some(visibility) => ConvexValue::try_from(visibility)?
            },
            "args" => serde_json::to_string(&args_json)?
        )
    }
}

impl TryFrom<AnalyzedFunction> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: AnalyzedFunction) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

impl TryFrom<ConvexObject> for AnalyzedFunction {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);
        let name = match fields.remove("name") {
            Some(ConvexValue::String(s)) => s.parse()?,
            v => anyhow::bail!("Invalid name field for AnalyzedFunction: {v:?}"),
        };
        let pos = match fields.remove("pos") {
            Some(ConvexValue::Object(o)) => Some(AnalyzedSourcePosition::try_from(o)?),
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid pos field for AnalyzedFunction: {v:?}"),
        };
        let udf_type = match fields.remove("udfType") {
            Some(ConvexValue::String(s)) => s.parse()?,
            v => anyhow::bail!("Invalid udfType for AnalyzedFunction: {v:?}"),
        };
        let visibility = match fields.remove("visibility") {
            Some(ConvexValue::Object(o)) => Some(Visibility::try_from(o)?),
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid visibility field for AnalyzedFunction: {v:?}"),
        };
        let args = match fields.remove("args") {
            Some(ConvexValue::String(s)) => {
                let deserialized_value: JsonValue = serde_json::from_str(&s)?;
                ArgsValidator::try_from(deserialized_value)?
            },
            // If this function was defined using the npm package before 0.13.0
            // there will be no args validator. Default to unvalidated.
            None => ArgsValidator::Unvalidated,
            v => anyhow::bail!("Invalid args field for AnalyzedFunction: {v:?}"),
        };

        Ok(Self {
            name,
            pos,
            udf_type,
            visibility,
            args,
        })
    }
}

struct HttpActionRoutePersisted(HttpActionRoute);

impl TryFrom<HttpActionRoutePersisted> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: HttpActionRoutePersisted) -> Result<Self, Self::Error> {
        obj!(
            "path" => value.0.path,
            "method" => value.0.method.to_string(),
        )
    }
}

impl TryFrom<HttpActionRoutePersisted> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: HttpActionRoutePersisted) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

impl TryFrom<ConvexObject> for HttpActionRoutePersisted {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);
        let path = match fields.remove("path") {
            Some(ConvexValue::String(s)) => s.into(),
            v => anyhow::bail!("Invalid path field for HttpActionRoute: {v:?}"),
        };
        let method = match fields.remove("method") {
            Some(ConvexValue::String(s)) => s.parse()?,
            v => anyhow::bail!("Invalid method for HttpActionRoute: {v:?}"),
        };
        Ok(Self(HttpActionRoute { path, method }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct AnalyzedHttpRoute {
    pub route: HttpActionRoute,
    pub pos: Option<AnalyzedSourcePosition>,
}

impl HeapSize for AnalyzedHttpRoute {
    fn heap_size(&self) -> usize {
        self.route.heap_size() + self.pos.heap_size()
    }
}
impl TryFrom<AnalyzedHttpRoute> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: AnalyzedHttpRoute) -> Result<Self, Self::Error> {
        obj!(
            "route" => HttpActionRoutePersisted(value.route),
            "pos" => match value.pos {
                None =>  ConvexValue::Null,
                Some(pos) => ConvexValue::try_from(pos)?,
            },
        )
    }
}

impl TryFrom<AnalyzedHttpRoute> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: AnalyzedHttpRoute) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

impl TryFrom<ConvexObject> for AnalyzedHttpRoute {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);
        let route = match fields.remove("route") {
            Some(ConvexValue::Object(o)) => HttpActionRoutePersisted::try_from(o)?.0,
            v => anyhow::bail!("Invalid route field for AnalyzedHttpRoute: {:?}", v),
        };
        let pos = match fields.remove("pos") {
            Some(ConvexValue::Object(pos)) => Some(pos.try_into()?),
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid lineno field for AnalyzedHttpRoute: {v:?}"),
        };
        Ok(Self { route, pos })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct MappedModule {
    // Index of the module's original source in the source map.
    // TODO: consider removing this or moving this out of MappedModule into AnalyzedModule and
    //  instead just include source information. This requires a decent migration from Dashboard
    //  schema.
    //  See https://github.com/get-convex/convex/pull/14382/files#r1252372646 for further discussion.
    pub source_index: Option<u32>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "value::heap_size::of(prop::collection::vec(any::<AnalyzedFunction>(), \
                        0..4))"
        )
    )]
    pub functions: WithHeapSize<Vec<AnalyzedFunction>>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "prop::option::of(value::heap_size::of(prop::collection::vec(any::<AnalyzedHttpRoute>(), 0..4)))"
        )
    )]
    pub http_routes: Option<WithHeapSize<Vec<AnalyzedHttpRoute>>>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "prop::option::of(value::heap_size::of(prop::collection::btree_map(any::<CronIdentifier>(), \
                        any::<CronSpec>(), 0..4)))"
        )
    )]
    pub cron_specs: Option<WithHeapSize<BTreeMap<CronIdentifier, CronSpec>>>,
}

impl HeapSize for MappedModule {
    fn heap_size(&self) -> usize {
        self.source_index.heap_size()
            + self.functions.heap_size()
            + self.http_routes.heap_size()
            + self.cron_specs.heap_size()
    }
}

impl TryFrom<MappedModule> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: MappedModule) -> Result<Self, Self::Error> {
        obj!(
            "sourceIndex" => match value.source_index {
                None => ConvexValue::Null,
                Some(index) => ConvexValue::from(index as i64),
            },
            "functions" => value
                .functions
                .into_iter()
                .map(ConvexValue::try_from)
                .try_collect::<Vec<_>>()?,
            "httpRoutes" => match value.http_routes {
                None => ConvexValue::Null,
                Some(http_routes) => http_routes
                    .into_iter()
                    .map(ConvexValue::try_from)
                    .try_collect::<Vec<_>>()?
                    .try_into()?,
            },
            "cronSpecs" => match value.cron_specs {
                None => ConvexValue::Null,
                Some(specs) => {
                    // Array of objects with { identifier: string, spec: CronSpec }
                    let mut arr: Vec<ConvexValue> = vec![];
                    for (identifier, cron_spec) in specs {
                        let spec_object = ConvexObject::try_from(cron_spec)?;
                        let identifier = ConvexValue::try_from(identifier.to_string())?;
                        let obj = obj!(
                            "identifier" => identifier,
                            "spec" => spec_object,
                        )?;
                        arr.push(ConvexValue::from(obj));
                    }
                    ConvexValue::try_from(arr)?
                }
            }
        )
    }
}

impl TryFrom<MappedModule> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: MappedModule) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

impl TryFrom<ConvexObject> for MappedModule {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);
        let source_index = match fields.remove("sourceIndex") {
            Some(ConvexValue::Int64(index)) => Some(index.try_into()?),
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid functions field for MappedModule: {v:?}"),
        };
        let functions = match fields.remove("functions") {
            Some(ConvexValue::Array(v)) => v
                .into_iter()
                .map(|v| AnalyzedFunction::try_from(ConvexObject::try_from(v)?))
                .collect::<anyhow::Result<WithHeapSize<_>>>()?,
            v => anyhow::bail!("Invalid functions field for MappedModule: {v:?}"),
        };
        let http_routes = match fields.remove("httpRoutes") {
            Some(ConvexValue::Array(v)) => {
                let mut routes: WithHeapSize<Vec<AnalyzedHttpRoute>> = WithHeapSize::default();
                for item in v.into_iter() {
                    let obj = ConvexObject::try_from(item)?;
                    let route: anyhow::Result<_> = AnalyzedHttpRoute::try_from(obj);
                    routes.push(route?);
                }
                Some(routes)
            },
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid httpRoutes field for MappedModule: {v:?}"),
        };
        let cron_specs = match fields.remove("cronSpecs") {
            Some(ConvexValue::Array(arr)) => {
                let mut specs: WithHeapSize<BTreeMap<CronIdentifier, CronSpec>> =
                    WithHeapSize::default();
                for item in arr {
                    let obj = ConvexObject::try_from(item)?;
                    let mut fields = BTreeMap::from(obj);
                    let identifier: CronIdentifier = match fields.remove("identifier") {
                        Some(ConvexValue::String(s)) => s.parse()?,
                        _ => anyhow::bail!("Invalid identifier field for cronSpecs"),
                    };
                    let spec: CronSpec = match fields.remove("spec") {
                        Some(ConvexValue::Object(o)) => CronSpec::try_from(o)?,
                        _ => anyhow::bail!("Invalid spec field for cronSpecs"),
                    };
                    specs.insert(identifier, spec);
                }
                Some(specs)
            },
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid cronSpecs field for MappedModule: {v:?}"),
        };
        Ok(Self {
            source_index,
            functions,
            http_routes,
            cron_specs,
        })
    }
}

impl TryFrom<ModuleVersionMetadata> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(m: ModuleVersionMetadata) -> anyhow::Result<Self> {
        obj!(
            "module_id" => m.module_id,
            "source" => m.source,
            "sourceMap" => m.source_map.map(ConvexValue::try_from).transpose()?.unwrap_or(ConvexValue::Null),
            "version" => m.version,
            "sourcePackageId" => m.source_package_id.map(ConvexValue::try_from).transpose()?.unwrap_or(ConvexValue::Null),
            "analyzeResult" => m.analyze_result.map(ConvexValue::try_from).transpose()?.unwrap_or(ConvexValue::Null),
            "environment" => m.environment.to_string(),
        )
    }
}

impl TryFrom<ModuleVersionMetadata> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: ModuleVersionMetadata) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

impl TryFrom<ConvexObject> for ModuleVersionMetadata {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();
        let module_id = match fields.remove("module_id") {
            Some(value) => value.try_into()?,
            v => anyhow::bail!("Invalid module_id field for ModuleVersionMetadata: {:?}", v),
        };
        let source_map = match fields.remove("sourceMap") {
            Some(ConvexValue::String(s)) => Some(s.into()),
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!("Invalid sourceMap field for ModuleVersionMetadata: {:?}", v),
        };
        let version = match fields.remove("version") {
            Some(ConvexValue::Int64(s)) => s,
            v => anyhow::bail!("Invalid version field for ModuleVersionMetadata: {:?}", v),
        };
        let source = match fields.remove("source") {
            Some(ConvexValue::String(s)) => s.into(),
            v => anyhow::bail!("Invalid source field: {v:?}"),
        };
        let source_package_id = match fields.remove("sourcePackageId") {
            Some(ConvexValue::Null) | None => None,
            Some(ConvexValue::String(s)) => Some(DocumentIdV6::decode(&s)?.into()),
            v => anyhow::bail!(
                "Invalid sourcePackageId field for ModuleVersionMetadata: {:?}",
                v
            ),
        };
        let analyze_result = match fields.remove("analyzeResult") {
            Some(ConvexValue::Object(o)) => Some(o.try_into()?),
            Some(ConvexValue::Null) | None => None,
            v => anyhow::bail!(
                "Invalid analyzeResult field for ModuleVersionMetadata: {:?}",
                v
            ),
        };
        let environment = match fields.remove("environment") {
            Some(ConvexValue::String(s)) => s.parse()?,
            v => anyhow::bail!(
                "Invalid environment field for ModuleVersionMetadata: {:?}",
                v
            ),
        };
        Ok(Self {
            module_id,
            source,
            source_package_id,
            source_map,
            version,
            analyze_result,
            environment,
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::{
        obj,
        ConvexObject,
    };

    use super::{
        AnalyzedFunction,
        ModuleVersionMetadata,
    };
    use crate::modules::args_validator::ArgsValidator;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_module_version_roundtrips(v in any::<ModuleVersionMetadata>()) {
            assert_roundtrips::<ModuleVersionMetadata, ConvexObject>(v);
        }
    }

    #[test]
    fn test_analyzed_function_backwards_compatibility() -> anyhow::Result<()> {
        // Old metadata won't have `visibility` or `args`
        let metadata: ConvexObject = obj!(
            "name" =>  "myFunction",
            "lineno" => 1,
            "udfType" => "Query"
        )?;
        let function = AnalyzedFunction::try_from(metadata)?;

        // Should parse as `visibility: None`, and `args: Unvalidated`.
        assert_eq!(function.visibility, None);
        assert_eq!(function.args, ArgsValidator::Unvalidated);
        Ok(())
    }
}
