use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    mem,
    ops::Deref,
    str::FromStr,
};

use async_lru::async_lru::SizedValue;
use common::types::{
    HttpActionRoute,
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
    CanonicalizedModulePath,
    FunctionName,
};
use value::heap_size::{
    HeapSize,
    WithHeapSize,
};

use super::function_validators::{
    ArgsValidator,
    ReturnsValidator,
};
use crate::cron_jobs::types::{
    CronIdentifier,
    CronSpec,
    SerializedCronSpec,
};

/// System-assigned version number for modules.
pub type ModuleVersion = i64;

/// User-specified JavaScript source code for a module.
pub type ModuleSource = String;

/// Bundler-generated source map for a `ModuleSource`.
pub type SourceMap = String;

#[derive(Debug, Clone)]
pub struct FullModuleSource {
    pub source: ModuleSource,
    pub source_map: Option<SourceMap>,
}

impl SizedValue for FullModuleSource {
    fn size(&self) -> u64 {
        (self.source.heap_size() + self.source_map.heap_size()) as u64
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
    pub http_routes: Option<AnalyzedHttpRoutes>,
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
                .map(|routes| {
                    let routes = routes.into_iter().map(TryFrom::try_from).try_collect()?;
                    AnalyzedHttpRoutes::new(routes)
                })
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

pub fn invalid_function_name_error(e: &anyhow::Error) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "InvalidFunctionName",
        format!("Invalid function name: {}", e),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct AnalyzedFunction {
    pub name: FunctionName,
    pub pos: Option<AnalyzedSourcePosition>,
    pub udf_type: UdfType,
    pub visibility: Option<Visibility>,
    pub args: ArgsValidator,
    pub returns: ReturnsValidator,
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
    returns: Option<String>,
}

impl TryFrom<AnalyzedFunction> for SerializedAnalyzedFunction {
    type Error = anyhow::Error;

    fn try_from(f: AnalyzedFunction) -> anyhow::Result<Self> {
        let args_json = JsonValue::try_from(f.args)?;
        let returns_json = JsonValue::try_from(f.returns)?;
        Ok(Self {
            name: f.name.to_string(),
            pos: f.pos.map(TryFrom::try_from).transpose()?,
            udf_type: f.udf_type.to_string(),
            visibility: f.visibility,
            args: Some(serde_json::to_string(&args_json)?),
            returns: Some(serde_json::to_string(&returns_json)?),
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
            returns: match f.returns {
                Some(returns) => {
                    let deserialized_value: JsonValue = serde_json::from_str(&returns)?;
                    ReturnsValidator::try_from(deserialized_value)?
                },
                None => ReturnsValidator::Unvalidated,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct AnalyzedHttpRoutes {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "value::heap_size::of(prop::collection::vec(any::<AnalyzedHttpRoute>(), \
                        0..4))"
        )
    )]
    routes: WithHeapSize<Vec<AnalyzedHttpRoute>>,
}

impl AnalyzedHttpRoutes {
    pub fn new(routes: Vec<AnalyzedHttpRoute>) -> anyhow::Result<Self> {
        // Parse routes into `(method, path)`.
        let mut prefix_routes = BTreeSet::new();
        let mut exact_routes = BTreeSet::new();
        for AnalyzedHttpRoute { route, .. } in &routes {
            let (set, path) = match route.path.strip_suffix('*') {
                Some(prefix_path) => {
                    if !prefix_path.starts_with('/') {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "BadHTTPRoute",
                            format!("Path {prefix_path:?} must start with a /")
                        ));
                    }
                    if !prefix_path.ends_with('/') {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "BadHTTPRoute",
                            format!("Path {prefix_path:?} must end with a /")
                        ));
                    }
                    (&mut prefix_routes, prefix_path)
                },
                None => {
                    if !route.path.starts_with('/') {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "BadHTTPRoute",
                            format!("Path {:?} must start with a /", route.path)
                        ));
                    }
                    (&mut exact_routes, &route.path[..])
                },
            };
            if !set.insert((&route.method, path)) {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "BadHTTPRoute",
                    format!("Duplicate HTTP route {path} for {}", route.method)
                ));
            }
        }
        Ok(Self {
            routes: routes.into(),
        })
    }
}

impl HeapSize for AnalyzedHttpRoutes {
    fn heap_size(&self) -> usize {
        self.routes.heap_size()
    }
}

impl IntoIterator for AnalyzedHttpRoutes {
    type IntoIter = Box<dyn Iterator<Item = AnalyzedHttpRoute>>;
    type Item = AnalyzedHttpRoute;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.routes.into_iter())
    }
}

impl Deref for AnalyzedHttpRoutes {
    type Target = [AnalyzedHttpRoute];

    fn deref(&self) -> &Self::Target {
        &self.routes
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
    pub http_routes: Option<AnalyzedHttpRoutes>,
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
                .map(|routes| {
                    let routes = routes.into_iter().map(TryFrom::try_from).try_collect()?;
                    AnalyzedHttpRoutes::new(routes)
                })
                .transpose()?,
            cron_specs: m
                .cron_specs
                .map(|specs| specs.into_iter().map(TryFrom::try_from).try_collect())
                .transpose()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use common::types::{
        HttpActionRoute,
        RoutableMethod,
    };
    use value::{
        obj,
        ConvexObject,
    };

    use super::AnalyzedFunction;
    use crate::modules::{
        function_validators::ArgsValidator,
        module_versions::{
            AnalyzedHttpRoute,
            AnalyzedHttpRoutes,
        },
    };

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

    #[test]
    fn test_http_routes() {
        let foo_prefix = AnalyzedHttpRoute {
            route: HttpActionRoute {
                path: "/foo/*".to_string(),
                method: RoutableMethod::Get,
            },
            pos: None,
        };
        let foo_bar_prefix = AnalyzedHttpRoute {
            route: HttpActionRoute {
                path: "/foo/bar/*".to_string(),
                method: RoutableMethod::Get,
            },
            pos: None,
        };
        let foo_exact = AnalyzedHttpRoute {
            route: HttpActionRoute {
                path: "/foo/".to_string(),
                method: RoutableMethod::Get,
            },
            pos: None,
        };
        let foo_exact_put = AnalyzedHttpRoute {
            route: HttpActionRoute {
                path: "/foo/".to_string(),
                method: RoutableMethod::Put,
            },
            pos: None,
        };

        // Fail when we have duplicate prefix routes.
        assert!(AnalyzedHttpRoutes::new(vec![foo_prefix.clone(), foo_prefix.clone()]).is_err());

        // Fail when we have duplicate exact routes.
        assert!(AnalyzedHttpRoutes::new(vec![foo_exact.clone(), foo_exact.clone()]).is_err());

        // Suceeed when we have an exact route that's a prefix of a prefix route.
        assert!(AnalyzedHttpRoutes::new(vec![foo_exact.clone(), foo_bar_prefix.clone()]).is_ok());

        // Succeed when we have exact routes with different methods.
        assert!(AnalyzedHttpRoutes::new(vec![foo_exact.clone(), foo_exact_put.clone()]).is_ok());
    }
}
