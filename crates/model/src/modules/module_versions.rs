use std::{
    collections::BTreeMap,
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
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use sync_types::{
    identifier::check_valid_identifier,
    CanonicalizedModulePath,
};
use value::{
    codegen_convex_serialization,
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    id_v6::DocumentIdV6,
    DeveloperDocumentId,
};

use super::args_validator::ArgsValidator;
use crate::{
    cron_jobs::types::{
        CronIdentifier,
        CronSpec,
        SerializedCronSpec,
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedAnalyzedModule {
    functions: Vec<SerializedAnalyzedFunction>,
    http_routes: Option<Vec<SerializedAnalyzedHttpRoute>>,
    cron_specs: Option<Vec<SerializedNamedCronSpec>>,
    source_mapped: Option<SerializedMappedModule>,
}

impl TryFrom<AnalyzedModule> for SerializedAnalyzedModule {
    type Error = anyhow::Error;

    fn try_from(m: AnalyzedModule) -> anyhow::Result<Self> {
        Ok(Self {
            functions: m
                .functions
                .into_iter()
                .map(TryFrom::try_from)
                .try_collect()?,
            http_routes: m
                .http_routes
                .map(|routes| routes.into_iter().map(TryFrom::try_from).try_collect())
                .transpose()?,
            cron_specs: m
                .cron_specs
                .map(|specs| specs.into_iter().map(TryFrom::try_from).try_collect())
                .transpose()?,
            source_mapped: m.source_mapped.map(TryFrom::try_from).transpose()?,
        })
    }
}

impl TryFrom<SerializedAnalyzedModule> for AnalyzedModule {
    type Error = anyhow::Error;

    fn try_from(m: SerializedAnalyzedModule) -> anyhow::Result<Self> {
        Ok(Self {
            functions: m
                .functions
                .into_iter()
                .map(TryFrom::try_from)
                .try_collect()?,
            http_routes: m
                .http_routes
                .map(|routes| routes.into_iter().map(TryFrom::try_from).try_collect())
                .transpose()?,
            cron_specs: m
                .cron_specs
                .map(|specs| specs.into_iter().map(TryFrom::try_from).try_collect())
                .transpose()?,
            source_mapped: m.source_mapped.map(TryFrom::try_from).transpose()?,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedNamedCronSpec {
    identifier: String,
    spec: SerializedCronSpec,
}

impl TryFrom<(CronIdentifier, CronSpec)> for SerializedNamedCronSpec {
    type Error = anyhow::Error;

    fn try_from((identifier, spec): (CronIdentifier, CronSpec)) -> anyhow::Result<Self> {
        Ok(Self {
            identifier: identifier.to_string(),
            spec: SerializedCronSpec::try_from(spec)?,
        })
    }
}

impl TryFrom<SerializedNamedCronSpec> for (CronIdentifier, CronSpec) {
    type Error = anyhow::Error;

    fn try_from(s: SerializedNamedCronSpec) -> anyhow::Result<Self> {
        Ok((s.identifier.parse()?, CronSpec::try_from(s.spec)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Visibility {
    Public,
    Internal,
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

#[derive(Serialize, Deserialize)]
// NOTE: serde not renamed to camelCase.
struct SerializedAnalyzedSourcePosition {
    path: String,
    start_lineno: u32,
    start_col: u32,
}

impl TryFrom<AnalyzedSourcePosition> for SerializedAnalyzedSourcePosition {
    type Error = anyhow::Error;

    fn try_from(p: AnalyzedSourcePosition) -> anyhow::Result<Self> {
        Ok(Self {
            path: p.path.as_str().to_string(),
            start_lineno: p.start_lineno,
            start_col: p.start_col,
        })
    }
}

impl TryFrom<SerializedAnalyzedSourcePosition> for AnalyzedSourcePosition {
    type Error = anyhow::Error;

    fn try_from(p: SerializedAnalyzedSourcePosition) -> anyhow::Result<Self> {
        Ok(Self {
            path: p.path.parse()?,
            start_lineno: p.start_lineno,
            start_col: p.start_col,
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedAnalyzedFunction {
    name: String,
    pos: Option<SerializedAnalyzedSourcePosition>,
    udf_type: String,
    visibility: Option<Visibility>,
    args: Option<String>,
}

impl TryFrom<AnalyzedFunction> for SerializedAnalyzedFunction {
    type Error = anyhow::Error;

    fn try_from(f: AnalyzedFunction) -> anyhow::Result<Self> {
        let args_json = JsonValue::try_from(f.args)?;
        Ok(Self {
            name: f.name.to_string(),
            pos: f.pos.map(TryFrom::try_from).transpose()?,
            udf_type: f.udf_type.to_string(),
            visibility: f.visibility,
            args: Some(serde_json::to_string(&args_json)?),
        })
    }
}

impl TryFrom<SerializedAnalyzedFunction> for AnalyzedFunction {
    type Error = anyhow::Error;

    fn try_from(f: SerializedAnalyzedFunction) -> anyhow::Result<Self> {
        Ok(Self {
            name: FunctionName::from_str(&f.name)?,
            pos: f.pos.map(AnalyzedSourcePosition::try_from).transpose()?,
            udf_type: f.udf_type.parse()?,
            visibility: f.visibility,
            args: match f.args {
                Some(args) => {
                    let deserialized_value: JsonValue = serde_json::from_str(&args)?;
                    ArgsValidator::try_from(deserialized_value)?
                },
                None => ArgsValidator::Unvalidated,
            },
        })
    }
}

mod codegen_analyzed_function {
    use value::codegen_convex_serialization;

    use super::{
        AnalyzedFunction,
        SerializedAnalyzedFunction,
    };

    codegen_convex_serialization!(AnalyzedFunction, SerializedAnalyzedFunction);
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedHttpActionRoute {
    path: String,
    method: String,
}

impl TryFrom<HttpActionRoute> for SerializedHttpActionRoute {
    type Error = anyhow::Error;

    fn try_from(r: HttpActionRoute) -> anyhow::Result<Self> {
        Ok(Self {
            path: r.path,
            method: r.method.to_string(),
        })
    }
}

impl TryFrom<SerializedHttpActionRoute> for HttpActionRoute {
    type Error = anyhow::Error;

    fn try_from(r: SerializedHttpActionRoute) -> anyhow::Result<Self> {
        Ok(Self {
            path: r.path,
            method: r.method.parse()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct AnalyzedHttpRoute {
    pub route: HttpActionRoute,
    pub pos: Option<AnalyzedSourcePosition>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedAnalyzedHttpRoute {
    route: SerializedHttpActionRoute,
    pos: Option<SerializedAnalyzedSourcePosition>,
}

impl HeapSize for AnalyzedHttpRoute {
    fn heap_size(&self) -> usize {
        self.route.heap_size() + self.pos.heap_size()
    }
}

impl TryFrom<AnalyzedHttpRoute> for SerializedAnalyzedHttpRoute {
    type Error = anyhow::Error;

    fn try_from(r: AnalyzedHttpRoute) -> anyhow::Result<Self> {
        Ok(Self {
            route: SerializedHttpActionRoute::try_from(r.route)?,
            pos: r.pos.map(TryFrom::try_from).transpose()?,
        })
    }
}

impl TryFrom<SerializedAnalyzedHttpRoute> for AnalyzedHttpRoute {
    type Error = anyhow::Error;

    fn try_from(r: SerializedAnalyzedHttpRoute) -> anyhow::Result<Self> {
        Ok(Self {
            route: HttpActionRoute::try_from(r.route)?,
            pos: r.pos.map(AnalyzedSourcePosition::try_from).transpose()?,
        })
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedMappedModule {
    source_index: Option<u32>,
    functions: Vec<SerializedAnalyzedFunction>,
    http_routes: Option<Vec<SerializedAnalyzedHttpRoute>>,
    cron_specs: Option<Vec<SerializedNamedCronSpec>>,
}

impl TryFrom<MappedModule> for SerializedMappedModule {
    type Error = anyhow::Error;

    fn try_from(m: MappedModule) -> anyhow::Result<Self> {
        Ok(Self {
            source_index: m.source_index,
            functions: m
                .functions
                .into_iter()
                .map(TryFrom::try_from)
                .try_collect()?,
            http_routes: m
                .http_routes
                .map(|routes| routes.into_iter().map(TryFrom::try_from).try_collect())
                .transpose()?,
            cron_specs: m
                .cron_specs
                .map(|specs| specs.into_iter().map(TryFrom::try_from).try_collect())
                .transpose()?,
        })
    }
}

impl TryFrom<SerializedMappedModule> for MappedModule {
    type Error = anyhow::Error;

    fn try_from(m: SerializedMappedModule) -> anyhow::Result<Self> {
        Ok(Self {
            source_index: m.source_index,
            functions: m
                .functions
                .into_iter()
                .map(TryFrom::try_from)
                .try_collect()?,
            http_routes: m
                .http_routes
                .map(|routes| routes.into_iter().map(TryFrom::try_from).try_collect())
                .transpose()?,
            cron_specs: m
                .cron_specs
                .map(|specs| specs.into_iter().map(TryFrom::try_from).try_collect())
                .transpose()?,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedModuleVersionMetadata {
    #[serde(rename = "module_id")]
    module_id: String,
    source: String,
    source_package_id: Option<String>,
    source_map: Option<String>,
    version: ModuleVersion,
    analyze_result: Option<SerializedAnalyzedModule>,
    environment: String,
}

impl TryFrom<ModuleVersionMetadata> for SerializedModuleVersionMetadata {
    type Error = anyhow::Error;

    fn try_from(m: ModuleVersionMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            module_id: m.module_id.encode(),
            source: m.source,
            source_package_id: m
                .source_package_id
                .map(|id| DeveloperDocumentId::from(id).encode()),
            source_map: m.source_map,
            version: m.version,
            analyze_result: m.analyze_result.map(TryFrom::try_from).transpose()?,
            environment: m.environment.to_string(),
        })
    }
}

impl TryFrom<SerializedModuleVersionMetadata> for ModuleVersionMetadata {
    type Error = anyhow::Error;

    fn try_from(m: SerializedModuleVersionMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            module_id: DocumentIdV6::decode(&m.module_id)?,
            source: m.source,
            source_package_id: m
                .source_package_id
                .map(|id| DocumentIdV6::decode(&id))
                .transpose()?
                .map(From::from),
            source_map: m.source_map,
            version: m.version,
            analyze_result: m.analyze_result.map(TryFrom::try_from).transpose()?,
            environment: m.environment.parse()?,
        })
    }
}

codegen_convex_serialization!(ModuleVersionMetadata, SerializedModuleVersionMetadata);

#[cfg(test)]
mod tests {
    use value::{
        obj,
        ConvexObject,
    };

    use super::AnalyzedFunction;
    use crate::modules::args_validator::ArgsValidator;

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
