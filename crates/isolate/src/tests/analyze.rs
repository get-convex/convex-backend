use std::{
    collections::BTreeMap,
    str::FromStr,
};

use common::{
    components::{
        CanonicalizedComponentModulePath,
        ComponentDefinitionId,
    },
    types::{
        ModuleEnvironment,
        RoutableMethod,
        UdfType,
    },
};
use keybroker::Identity;
use maplit::btreemap;
use model::{
    config::types::ModuleConfig,
    cron_jobs::types::{
        CronIdentifier,
        CronSchedule,
        CronSpec,
    },
    modules::{
        args_validator::ArgsValidator,
        module_versions::{
            AnalyzedFunction,
            AnalyzedSourcePosition,
            Visibility,
        },
        ModuleModel,
    },
    udf_config::types::UdfConfig,
};
use pretty_assertions::assert_eq;
use runtime::testing::TestRuntime;
use value::{
    assert_obj,
    ConvexArray,
    ConvexValue,
};

use crate::test_helpers::UdfTest;

#[convex_macro::test_runtime]
async fn test_analyze_module(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let modules = {
        let mut tx = t.database.begin(Identity::system()).await?;
        ModuleModel::new(&mut tx)
            .get_application_modules(ComponentDefinitionId::Root)
            .await?
    };

    let has_http = {
        let mut tx = t.database.begin(Identity::system()).await?;
        ModuleModel::new(&mut tx).has_http().await?
    };
    assert!(has_http);

    let udf_config = UdfConfig::new_for_test(&t.rt, "1000.0.0".parse()?);
    let mut result = t
        .isolate
        .analyze(udf_config.clone(), modules, BTreeMap::new())
        .await??;
    let analyze_path = CanonicalizedComponentModulePath {
        component: ComponentDefinitionId::Root,
        module_path: "analyze.js".parse()?,
    };
    let module = result.remove(&analyze_path).unwrap();

    let expected = [
        // name, expected_type, mapped_lineno
        ("g", UdfType::Mutation, 8),
        ("f1", UdfType::Mutation, 13),
        ("f2", UdfType::Mutation, 13),
        ("default", UdfType::Query, 20),
        ("h", UdfType::Query, 20),
        ("action_in_v8", UdfType::Action, 28),
    ];
    assert_eq!(module.functions.len(), expected.len());
    let source_mapped = module.source_mapped.as_ref().unwrap();
    assert_eq!(source_mapped.functions.len(), expected.len());

    for (i, (name, expected_type, mapped_lineno)) in expected.iter().enumerate() {
        let function = &module.functions[i];
        assert_eq!(&function.name[..], *name);
        assert_eq!(&function.udf_type, expected_type);

        let mapped_function = &source_mapped.functions[i];
        assert_eq!(&mapped_function.name[..], *name);
        assert_eq!(
            mapped_function.pos.as_ref().unwrap().start_lineno,
            *mapped_lineno
        );
        assert_eq!(&mapped_function.udf_type, expected_type);
    }

    let http_path = CanonicalizedComponentModulePath {
        component: ComponentDefinitionId::Root,
        module_path: "http.js".parse()?,
    };
    let module = result.remove(&http_path).unwrap();

    let expected = vec![
        // name, expected_type, mapped_lineno
        ("erroringQuery", UdfType::Query, 37),
    ];
    assert_eq!(module.functions.len(), expected.len());
    let expected_routes_unmapped = vec![
        ("/imported", RoutableMethod::Get),
        ("/separate_function", RoutableMethod::Get),
        ("/inline", RoutableMethod::Get),
    ];
    let expected_routes_mapped = vec![
        ("/imported", RoutableMethod::Get, None),
        (
            "/separate_function",
            RoutableMethod::Get,
            Some(AnalyzedSourcePosition {
                path: "http.js".parse()?,
                start_lineno: 12,
                start_col: 26,
            }),
        ),
        (
            "/inline",
            RoutableMethod::Get,
            Some(AnalyzedSourcePosition {
                path: "http.js".parse()?,
                start_lineno: 26,
                start_col: 20,
            }),
        ),
    ];
    assert_eq!(
        module
            .http_routes
            .as_ref()
            .expect("no analyzed http_routes found")
            .len(),
        expected_routes_unmapped.len()
    );
    let source_mapped = module.source_mapped.as_ref().unwrap();
    assert!(source_mapped.http_routes.is_some());
    assert_eq!(
        module.http_routes.as_ref().unwrap().len(),
        source_mapped.http_routes.as_ref().unwrap().len()
    );
    for (i, (path, method)) in expected_routes_unmapped.iter().enumerate() {
        let route = &module.http_routes.as_ref().unwrap()[i];
        assert_eq!(&route.route.path, path);
        assert_eq!(&route.route.method, method);
    }

    for (i, (path, method, mapped_pos)) in expected_routes_mapped.iter().enumerate() {
        let mapped_route = &source_mapped
            .http_routes
            .as_ref()
            .expect("no mapped http_routes found")[i];
        assert_eq!(&mapped_route.route.path, path);
        assert_eq!(&mapped_route.route.method, method);
        assert_eq!(mapped_route.pos.as_ref(), mapped_pos.as_ref());
    }

    let crons_path = CanonicalizedComponentModulePath {
        component: ComponentDefinitionId::Root,
        module_path: "crons.js".parse()?,
    };
    let module = result.remove(&crons_path).unwrap();
    let arg = assert_obj!(
       "x" => ConvexValue::Float64(1.0)
    );
    let args: ConvexArray = vec![ConvexValue::Object(arg)].try_into()?;
    assert_eq!(
        module.cron_specs,
        Some(btreemap!(
        CronIdentifier::from_str("weekly re-engagement email")? => CronSpec {
            udf_path: "crons.js:addOne".parse()?,
            udf_args: args.clone(),
            cron_schedule: CronSchedule::Weekly { day_of_week: 2, hour_utc: 17, minute_utc: 30 }},
        CronIdentifier::from_str("add one every hour")? => CronSpec {
            udf_path: "crons.js:addOne".parse()?,
            udf_args: args.clone(),
            cron_schedule: CronSchedule::Interval{ seconds: 3600 * 24 * 7 } },
        CronIdentifier::from_str("clear presence data")? => CronSpec {
            udf_path: "crons.js:addOne".parse()?,
            udf_args: args,
            cron_schedule: CronSchedule::Interval{ seconds: 300} },
        ).into()),
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_analyze_http_errors(rt: TestRuntime) -> anyhow::Result<()> {
    let cases = [
        // No default export in http.js
        ("http_no_default.js", "must have a default export"),
        // default export is not an object (TODO)
        (
            "http_undefined_default.js",
            "The default export of `convex/http.js` is not a Router.",
        ),
        // default export is not a router
        (
            "http_object_default.js",
            "The default export of `convex/http.js` is not a Router.",
        ),
    ];

    let t = UdfTest::default(rt).await?;

    for (file, expected_error) in cases {
        let mut modules = {
            let mut tx = t.database.begin(Identity::system()).await?;
            ModuleModel::new(&mut tx)
                .get_application_modules(ComponentDefinitionId::Root)
                .await?
        };

        // Analyze this file as though it were the router (normally http.js)
        let test_http_canonical = CanonicalizedComponentModulePath {
            component: ComponentDefinitionId::Root,
            module_path: file.parse()?,
        };

        let real_http = CanonicalizedComponentModulePath {
            component: ComponentDefinitionId::Root,
            module_path: "http.js".parse()?,
        };
        modules.remove(&real_http).unwrap();
        let test_http_module: ModuleConfig = modules.remove(&test_http_canonical).unwrap();

        // stick in an `is_http: true` module with the name of the module we're testing
        let with_http = CanonicalizedComponentModulePath {
            component: ComponentDefinitionId::Root,
            module_path: test_http_canonical.module_path.with_http(),
        };
        modules.insert(with_http, test_http_module.clone());

        // reinsert the original module so it's not missing
        modules.insert(test_http_canonical, test_http_module);

        let udf_config = UdfConfig::new_for_test(&t.rt, "1000.0.0".parse()?);
        let Err(err) = t
            .isolate
            .analyze(udf_config, modules, BTreeMap::new())
            .await?
        else {
            anyhow::bail!("No JsError raised for missing default export");
        };
        assert!(format!("{}", err).contains(expected_error), "{err:?}");
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_analyze_function(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let modules = {
        let mut tx = t.database.begin(Identity::system()).await?;
        ModuleModel::new(&mut tx)
            .get_application_modules(ComponentDefinitionId::Root)
            .await?
    };

    let udf_config = UdfConfig::new_for_test(&t.rt, "1000.0.0".parse()?);
    let mut result = t
        .isolate
        .analyze(udf_config, modules, BTreeMap::new())
        .await??;
    let source_maps_path = CanonicalizedComponentModulePath {
        component: ComponentDefinitionId::Root,
        module_path: "sourceMaps.js".parse()?,
    };
    let analyzed_module = result.remove(&source_maps_path).unwrap();

    assert_eq!(
        &Vec::from(analyzed_module.functions.clone()),
        &[
            AnalyzedFunction {
                name: "throwsError".parse()?,
                // Don't check line numbers since those change on every `convex/server`
                // change.
                pos: Some(AnalyzedSourcePosition {
                    path: "sourceMaps.js".parse()?,
                    start_lineno: analyzed_module.functions[0]
                        .pos
                        .as_ref()
                        .unwrap()
                        .start_lineno,
                    start_col: analyzed_module.functions[0].pos.as_ref().unwrap().start_col,
                }),
                udf_type: UdfType::Query,
                visibility: Some(Visibility::Public),
                args: ArgsValidator::Unvalidated
            },
            AnalyzedFunction {
                name: "throwsErrorInDep".parse()?,
                pos: Some(AnalyzedSourcePosition {
                    path: "sourceMaps.js".parse()?,
                    start_lineno: analyzed_module.functions[1]
                        .pos
                        .as_ref()
                        .unwrap()
                        .start_lineno,
                    start_col: analyzed_module.functions[1].pos.as_ref().unwrap().start_col,
                }),
                udf_type: UdfType::Query,
                visibility: Some(Visibility::Public),
                args: ArgsValidator::Unvalidated
            },
        ],
    );
    let source_mapped = analyzed_module.source_mapped.unwrap();
    assert_eq!(
        &Vec::from(source_mapped.functions),
        &[
            AnalyzedFunction {
                name: "throwsError".parse()?,
                pos: Some(AnalyzedSourcePosition {
                    path: "sourceMaps.js".parse()?,
                    start_lineno: 21,
                    start_col: analyzed_module.functions[0].pos.as_ref().unwrap().start_col,
                }),
                udf_type: UdfType::Query,
                visibility: Some(Visibility::Public),
                args: ArgsValidator::Unvalidated
            },
            AnalyzedFunction {
                name: "throwsErrorInDep".parse()?,
                pos: Some(AnalyzedSourcePosition {
                    path: "sourceMaps.js".parse()?,
                    start_lineno: 27,
                    start_col: analyzed_module.functions[1].pos.as_ref().unwrap().start_col,
                }),
                udf_type: UdfType::Query,
                visibility: Some(Visibility::Public),
                args: ArgsValidator::Unvalidated
            },
        ],
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_analyze_internal_function(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let modules = {
        let mut tx = t.database.begin(Identity::system()).await?;
        ModuleModel::new(&mut tx)
            .get_application_modules(ComponentDefinitionId::Root)
            .await?
    };

    let udf_config = UdfConfig::new_for_test(&t.rt, "1000.0.0".parse()?);
    let mut result = t
        .isolate
        .analyze(udf_config, modules, BTreeMap::new())
        .await??;
    let internal_path = CanonicalizedComponentModulePath {
        component: ComponentDefinitionId::Root,
        module_path: "internal.js".parse()?,
    };
    let analyzed_module = result.remove(&internal_path).unwrap();

    assert_eq!(
        &Vec::from(analyzed_module.functions.clone()),
        &[
            AnalyzedFunction {
                name: "myInternalQuery".parse()?,
                // Don't check line numbers since those change on every `convex/server`
                // change.
                pos: analyzed_module.functions[0].pos.clone(),
                udf_type: UdfType::Query,
                visibility: Some(Visibility::Internal),
                args: ArgsValidator::Unvalidated
            },
            AnalyzedFunction {
                name: "publicQuery".parse()?,
                // Don't check line numbers since those change on every `convex/server`
                // change.
                pos: analyzed_module.functions[1].pos.clone(),
                udf_type: UdfType::Query,
                visibility: Some(Visibility::Public),
                args: ArgsValidator::Unvalidated
            },
            AnalyzedFunction {
                name: "myInternalMutation".parse()?,
                // Don't check line numbers since those change on every `convex/server`
                // change.
                pos: analyzed_module.functions[2].pos.clone(),
                udf_type: UdfType::Mutation,
                visibility: Some(Visibility::Internal),
                args: ArgsValidator::Unvalidated
            },
            AnalyzedFunction {
                name: "publicMutation".parse()?,
                // Don't check line numbers since those change on every `convex/server`
                // change.
                pos: analyzed_module.functions[3].pos.clone(),
                udf_type: UdfType::Mutation,
                visibility: Some(Visibility::Public),
                args: ArgsValidator::Unvalidated
            },
        ],
    );
    let source_mapped = analyzed_module.source_mapped.unwrap();
    assert_eq!(
        &Vec::from(source_mapped.functions.clone()),
        &[
            AnalyzedFunction {
                name: "myInternalQuery".parse()?,
                pos: Some(AnalyzedSourcePosition {
                    path: "internal.js".parse()?,
                    start_lineno: 16,
                    start_col: analyzed_module.functions[0].pos.as_ref().unwrap().start_col,
                }),
                udf_type: UdfType::Query,
                visibility: Some(Visibility::Internal),
                args: ArgsValidator::Unvalidated
            },
            AnalyzedFunction {
                name: "publicQuery".parse()?,
                pos: Some(AnalyzedSourcePosition {
                    path: "internal.js".parse()?,
                    start_lineno: 19,
                    start_col: analyzed_module.functions[1].pos.as_ref().unwrap().start_col,
                }),
                udf_type: UdfType::Query,
                visibility: Some(Visibility::Public),
                args: ArgsValidator::Unvalidated
            },
            AnalyzedFunction {
                name: "myInternalMutation".parse()?,
                pos: Some(AnalyzedSourcePosition {
                    path: "internal.js".parse()?,
                    start_lineno: 23,
                    start_col: analyzed_module.functions[2].pos.as_ref().unwrap().start_col,
                }),
                udf_type: UdfType::Mutation,
                visibility: Some(Visibility::Internal),
                args: ArgsValidator::Unvalidated
            },
            AnalyzedFunction {
                name: "publicMutation".parse()?,
                pos: Some(AnalyzedSourcePosition {
                    path: "internal.js".parse()?,
                    start_lineno: 26,
                    start_col: analyzed_module.functions[3].pos.as_ref().unwrap().start_col,
                }),
                udf_type: UdfType::Mutation,
                visibility: Some(Visibility::Public),
                args: ArgsValidator::Unvalidated
            },
        ],
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_analyze_developer_errors(rt: TestRuntime) -> anyhow::Result<()> {
    let cases = [
        // Syntax errors should be propagated back to the developer.
        ("const x = 'what", "SyntaxError"),
        // `esbuild` should catch most import errors, but we should still degrade gracefully if we
        // see an import error at this layer.
        (
            "import { something } from 'nonexistent';",
            r#"Relative import path "nonexistent" not prefixed with /"#,
        ),
        (
            "import { something } from './nonexistent';",
            "Couldn't find JavaScript module",
        ),
        // Throwing an error within a syntactically valid module is still a developer error.  The
        // error message is a bit jank, but hopefully it's good enough for now to point developers
        // to their errors.
        // ```
        // Uncaught Error: Uncaught Error: no thanks
        //   at <anonymous> (convex:/broken.js:1:7)
        //   at <anonymous> (convex:/_system/cli/listModules.js:14:27)
        //   at async invokeQuery (convex:/_system/_deps/HBQGL2NV.js:774:18)
        //
        //   at <anonymous> (convex:/_system/cli/listModules.js:14:27)
        //   at async invokeQuery (convex:/_system/_deps/HBQGL2NV.js:774:18)
        // ```
        ("throw new Error('no thanks');", "Uncaught Error: no thanks"),
        (
            r##"Convex.syscall("insert", JSON.stringify({ table: "oh", value: { hello: "there" } }))"##,
            "Can't use database at import time",
        ),
        (
            "async function test(){}; await test();",
            "Top-level awaits in source files are unsupported",
        ),
    ];

    for (source, expected_error) in cases {
        let module = ModuleConfig {
            path: "broken.js".parse()?,
            source: source.to_owned(),
            source_map: None,
            environment: ModuleEnvironment::Isolate,
        };
        let Err(err) = UdfTest::default_with_modules(vec![module], rt.clone()).await? else {
            anyhow::bail!("No JsError raised for broken source: {}", source);
        };
        assert!(format!("{}", err).contains(expected_error), "{err:?}");
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_analyze_imports_are_none(rt: TestRuntime) -> anyhow::Result<()> {
    // Tests that imported handler methods report None for
    // Option<AnalyzedSourcePosition>. In the future, we might want to also
    // report the file itself, and s
    let t = UdfTest::default(rt).await?;
    let cases = [
        (
            "http_all_imported_handlers.js",
            vec![
                ("/test1", None),
                ("/test2", None),
                ("/test3", None),
                ("/test4", None),
            ],
        ),
        ("http.js", vec![("/imported", None)]),
        (
            "http_no_imports.js",
            vec![(
                "/test",
                Some(AnalyzedSourcePosition {
                    path: "http_no_imports.js".parse()?,
                    start_lineno: 9,
                    start_col: 15,
                }),
            )],
        ),
    ];

    for (case, expected) in cases {
        // Construct the http.js module for analysis
        let http_path = CanonicalizedComponentModulePath {
            component: ComponentDefinitionId::Root,
            module_path: "http.js".parse()?,
        };

        let mut modules = {
            let mut tx = t.database.begin(Identity::system()).await?;
            ModuleModel::new(&mut tx)
                .get_application_modules(ComponentDefinitionId::Root)
                .await?
        };

        // Reinsert the case as http.js, replacing the old http.js, so that the
        // http_analyze codepath is used on this file.
        let case_canon_path = CanonicalizedComponentModulePath {
            component: ComponentDefinitionId::Root,
            module_path: case.parse()?,
        };
        let module_config = modules
            .remove(&case_canon_path)
            .expect("Could not find case in list of modules");
        // For any file that is not http.js/ts, we need to remove the original http.js
        // and reinsert the path with http
        if !case_canon_path.module_path.is_http() {
            modules
                .remove(&http_path)
                .expect("Could not find original http.js");
            // Reinsert with and without http
            let with_http = CanonicalizedComponentModulePath {
                component: ComponentDefinitionId::Root,
                module_path: case_canon_path.module_path.with_http(),
            };
            modules.insert(with_http, module_config.clone()); // Reinsertion
        }
        // Reinsert original path so analysis doesn't complain it's missing
        modules.insert(case_canon_path.clone(), module_config);

        // Run analysis
        let udf_config = UdfConfig::new_for_test(&t.rt, "1000.0.0".parse()?);
        let mut analyze_result = t
            .isolate
            .analyze(udf_config, modules, BTreeMap::new())
            .await?
            .expect("analyze failed");
        let with_http = CanonicalizedComponentModulePath {
            component: ComponentDefinitionId::Root,
            module_path: case_canon_path.module_path.with_http(),
        };
        let module = analyze_result
            .remove(&with_http)
            .expect("could not find result for path with http");

        // Verify routes and lineno match
        let routes = module
            .http_routes
            .expect("http_routes in source_mapped was None");
        let route_map: BTreeMap<_, _> = routes
            .into_iter()
            .map(|f| (f.route.path.clone(), f.pos))
            .collect();

        for (route, pos) in expected.into_iter() {
            assert_eq!(
                pos.as_ref(),
                route_map
                    .get(route)
                    .expect("could not find route in route_map")
                    .as_ref()
            );
        }
    }

    Ok(())
}
