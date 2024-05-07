use common::types::{
    ModuleEnvironment,
    UdfType,
};
use model::{
    config::types::ModuleConfig,
    udf_config::types::UdfConfig,
};
use runtime::prod::ProdRuntime;

use crate::{
    test_helpers::ApplicationTestExt,
    tests::NODE_SOURCE,
    Application,
};

const ISOLATE_SOURCE: &str = r#"
export function isolateFunction() {}
isolateFunction.isRegistered = true;
isolateFunction.isQuery = true;
isolateFunction.invokeQuery = () => {};
"#;

const CRONS_SOURCE_A: &str = r#"
export default {
  isCrons: true,
  export() {
    return JSON.stringify({
      "an action": {
        name: "b:nodeFunction",
        args: [],
        schedule: { type: "interval", seconds: 1 },
      },
    });
  },
};
"#;

const CRONS_SOURCE_B: &str = r#"
export default {
  isCrons: true,
  export() {
    return JSON.stringify({
      "a mutation": {
        name: "a:isolateFunction",
        args: [],
        schedule: { type: "interval", seconds: 1 },
      },
    });
  },
};
"#;

// This test requires prod runtime since it analyzes node modules.
#[convex_macro::prod_rt_test]
async fn test_analyze(rt: ProdRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let modules = vec![
        ModuleConfig {
            path: "a.js".parse()?,
            source: ISOLATE_SOURCE.to_owned(),
            source_map: None,
            environment: ModuleEnvironment::Isolate,
        },
        ModuleConfig {
            path: "b.js".parse()?,
            source: NODE_SOURCE.to_owned(),
            source_map: None,
            environment: ModuleEnvironment::Node,
        },
    ];
    let source_package = application.upload_package(&modules, None).await?;
    let udf_config = UdfConfig::new_for_test(&rt, "1000.0.0".parse()?);
    let modules = application
        .analyze(udf_config, modules, source_package)
        .await??;
    assert_eq!(modules.len(), 2);

    assert_eq!(modules[&"a.js".parse()?].functions.len(), 1);
    let module = &modules[&"a.js".parse()?].functions[0];
    assert_eq!(module.udf_type, UdfType::Query);
    assert_eq!(&module.name[..], "isolateFunction");
    assert!(module.pos.is_none());

    assert_eq!(modules[&"b.js".parse()?].functions.len(), 1);
    let module = &modules[&"b.js".parse()?].functions[0];
    assert_eq!(module.udf_type, UdfType::Action);
    assert_eq!(&module.name[..], "nodeFunction");
    assert!(module.pos.is_none());

    Ok(())
}

#[convex_macro::prod_rt_test]
async fn test_analyze_with_source_map(rt: ProdRuntime) -> anyhow::Result<()> {
    const SAMPLE_SOURCE: &str = r#"
async function invokeAction(func, requestId, argsStr) {
  throw new Error("unimplemented");
}
var actionGeneric = func => {
  const q = func;
  if (q.isRegistered) {
    throw new Error("Function registered twice " + func);
  }
  q.isRegistered = true;
  q.isAction = true;
  q.isPublic = true;
  q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);
  return q;
};
var internalActionGeneric = func => {
    const q = func;
    if (q.isRegistered) {
      throw new Error("Function registered twice " + func);
    }
    q.isRegistered = true;
    q.isAction = true;
    q.isInternal = true;
    q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);
    return q;
  };
var action = actionGeneric;
var internalAction = internalActionGeneric;
var hello = action(async ({}) => {
  console.log("analyze me pls");
});
var internalHello = internalAction(async ({}) => {
  console.log("analyze me pls");
});
export { hello, internalHello };
"#;

    // Generated via `npx esbuild static_node_source.js --bundle --format=esm
    // --target=esnext --sourcemap=linked --outfile=out.js`
    const NODE_SOURCE_MAP: &str = r#"
{
  "version": 3,
  "sources": ["node_source.js"],
  "sourcesContent": ["async function invokeAction(func, requestId, argsStr) {\n  throw new Error(\"unimplemented\");\n}\nvar actionGeneric = func => {\n  const q = func;\n  if (q.isRegistered) {\n    throw new Error(\"Function registered twice \" + func);\n  }\n  q.isRegistered = true;\n  q.isAction = true;\n  q.isPublic = true;\n  q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);\n  return q;\n};\nvar internalActionGeneric = func => {\n    const q = func;\n    if (q.isRegistered) {\n      throw new Error(\"Function registered twice \" + func);\n    }\n    q.isRegistered = true;\n    q.isAction = true;\n    q.isInternal = true;\n    q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);\n    return q;\n  };\nvar action = actionGeneric;\nvar internalAction = internalActionGeneric;\nvar hello = action(async ({}) => {\n  console.log(\"analyze me pls\");\n});\nvar internalHello = internalAction(async ({}) => {\n  console.log(\"analyze me pls\");\n});\nexport { hello, internalHello };\n"],
  "mappings": ";AAAA,eAAe,aAAa,MAAM,WAAW,SAAS;AACpD,QAAM,IAAI,MAAM,eAAe;AACjC;AACA,IAAI,gBAAgB,UAAQ;AAC1B,QAAM,IAAI;AACV,MAAI,EAAE,cAAc;AAClB,UAAM,IAAI,MAAM,+BAA+B,IAAI;AAAA,EACrD;AACA,IAAE,eAAe;AACjB,IAAE,WAAW;AACb,IAAE,WAAW;AACb,IAAE,eAAe,CAAC,WAAW,YAAY,aAAa,MAAM,WAAW,OAAO;AAC9E,SAAO;AACT;AACA,IAAI,wBAAwB,UAAQ;AAChC,QAAM,IAAI;AACV,MAAI,EAAE,cAAc;AAClB,UAAM,IAAI,MAAM,+BAA+B,IAAI;AAAA,EACrD;AACA,IAAE,eAAe;AACjB,IAAE,WAAW;AACb,IAAE,aAAa;AACf,IAAE,eAAe,CAAC,WAAW,YAAY,aAAa,MAAM,WAAW,OAAO;AAC9E,SAAO;AACT;AACF,IAAI,SAAS;AACb,IAAI,iBAAiB;AACrB,IAAI,QAAQ,OAAO,OAAO,CAAC,MAAM;AAC/B,UAAQ,IAAI,gBAAgB;AAC9B,CAAC;AACD,IAAI,gBAAgB,eAAe,OAAO,CAAC,MAAM;AAC/C,UAAQ,IAAI,gBAAgB;AAC9B,CAAC;",
  "names": []
}
"#;

    const ISOLATE_SOURCE_MAP: &str = r#"
{
  "version": 3,
  "sources": ["isolate_source.js"],
  "sourcesContent": ["async function invokeAction(func, requestId, argsStr) {\n  throw new Error(\"unimplemented\");\n}\nvar actionGeneric = func => {\n  const q = func;\n  if (q.isRegistered) {\n    throw new Error(\"Function registered twice \" + func);\n  }\n  q.isRegistered = true;\n  q.isAction = true;\n  q.isPublic = true;\n  q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);\n  return q;\n};\nvar internalActionGeneric = func => {\n    const q = func;\n    if (q.isRegistered) {\n      throw new Error(\"Function registered twice \" + func);\n    }\n    q.isRegistered = true;\n    q.isAction = true;\n    q.isInternal = true;\n    q.invokeAction = (requestId, argsStr) => invokeAction(func, requestId, argsStr);\n    return q;\n  };\nvar action = actionGeneric;\nvar internalAction = internalActionGeneric;\nvar hello = action(async ({}) => {\n  console.log(\"analyze me pls\");\n});\nvar internalHello = internalAction(async ({}) => {\n  console.log(\"analyze me pls\");\n});\nexport { hello, internalHello };\n"],
  "mappings": ";AAAA,eAAe,aAAa,MAAM,WAAW,SAAS;AACpD,QAAM,IAAI,MAAM,eAAe;AACjC;AACA,IAAI,gBAAgB,UAAQ;AAC1B,QAAM,IAAI;AACV,MAAI,EAAE,cAAc;AAClB,UAAM,IAAI,MAAM,+BAA+B,IAAI;AAAA,EACrD;AACA,IAAE,eAAe;AACjB,IAAE,WAAW;AACb,IAAE,WAAW;AACb,IAAE,eAAe,CAAC,WAAW,YAAY,aAAa,MAAM,WAAW,OAAO;AAC9E,SAAO;AACT;AACA,IAAI,wBAAwB,UAAQ;AAChC,QAAM,IAAI;AACV,MAAI,EAAE,cAAc;AAClB,UAAM,IAAI,MAAM,+BAA+B,IAAI;AAAA,EACrD;AACA,IAAE,eAAe;AACjB,IAAE,WAAW;AACb,IAAE,aAAa;AACf,IAAE,eAAe,CAAC,WAAW,YAAY,aAAa,MAAM,WAAW,OAAO;AAC9E,SAAO;AACT;AACF,IAAI,SAAS;AACb,IAAI,iBAAiB;AACrB,IAAI,QAAQ,OAAO,OAAO,CAAC,MAAM;AAC/B,UAAQ,IAAI,gBAAgB;AAC9B,CAAC;AACD,IAAI,gBAAgB,eAAe,OAAO,CAAC,MAAM;AAC/C,UAAQ,IAAI,gBAAgB;AAC9B,CAAC;",
  "names": []
}
"#;

    let application = Application::new_for_tests(&rt).await?;
    let modules = vec![
        ModuleConfig {
            path: "isolate_source.js".parse()?,
            source: SAMPLE_SOURCE.to_string(),
            source_map: Some(ISOLATE_SOURCE_MAP.to_string()),
            environment: ModuleEnvironment::Isolate,
        },
        ModuleConfig {
            path: "node_source.js".parse()?,
            source: SAMPLE_SOURCE.to_string(),
            source_map: Some(NODE_SOURCE_MAP.to_string()),
            environment: ModuleEnvironment::Node,
        },
    ];

    let source_package = application.upload_package(&modules, None).await?;
    let udf_config = UdfConfig::new_for_test(&rt, "1000.0.0".parse()?);
    let modules = application
        .analyze(udf_config.clone(), modules, source_package)
        .await??;

    assert_eq!(modules.len(), 2);

    assert_eq!(modules[&"isolate_source.js".parse()?].functions.len(), 2);
    let module = &modules[&"isolate_source.js".parse()?];
    assert_eq!(&module.functions[0].name[..], "hello");
    assert_eq!(module.functions[0].udf_type, UdfType::Action);
    assert_eq!(module.functions[0].pos.as_ref().unwrap().start_lineno, 28);
    assert_eq!(&module.functions[1].name[..], "internalHello");
    assert_eq!(module.functions[1].udf_type, UdfType::Action);
    assert_eq!(module.functions[1].pos.as_ref().unwrap().start_lineno, 31);

    assert_eq!(modules[&"node_source.js".parse()?].functions.len(), 2);
    let module = &modules[&"node_source.js".parse()?];
    assert_eq!(&module.functions[0].name[..], "hello");
    assert_eq!(module.functions[0].udf_type, UdfType::Action);
    assert_eq!(module.functions[0].pos.as_ref().unwrap().start_lineno, 28);
    assert_eq!(&module.functions[1].name[..], "internalHello");
    assert_eq!(module.functions[1].udf_type, UdfType::Action);
    assert_eq!(module.functions[1].pos.as_ref().unwrap().start_lineno, 31);

    Ok(())
}

// This test requires prod runtime since it analyzes node modules.
#[convex_macro::prod_rt_test]
async fn test_analyze_crons(rt: ProdRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let modules = vec![
        ModuleConfig {
            path: "a.js".parse()?,
            source: ISOLATE_SOURCE.to_owned(),
            source_map: None,
            environment: ModuleEnvironment::Isolate,
        },
        ModuleConfig {
            path: "b.js".parse()?,
            source: NODE_SOURCE.to_owned(),
            source_map: None,
            environment: ModuleEnvironment::Node,
        },
        ModuleConfig {
            path: "crons.js".parse()?,
            source: CRONS_SOURCE_A.to_owned(),
            source_map: None,
            environment: ModuleEnvironment::Isolate,
        },
    ];
    let source_package = application.upload_package(&modules, None).await?;
    let udf_config = UdfConfig::new_for_test(&rt, "1000.0.0".parse()?);
    let modules = application
        .analyze(udf_config.clone(), modules, source_package)
        .await??;
    assert_eq!(modules.len(), 3);

    assert_eq!(modules[&"a.js".parse()?].functions.len(), 1);
    let module = &modules[&"a.js".parse()?].functions[0];
    assert_eq!(module.udf_type, UdfType::Query);
    assert_eq!(&module.name[..], "isolateFunction");
    assert!(module.pos.is_none());

    assert_eq!(modules[&"b.js".parse()?].functions.len(), 1);
    let module = &modules[&"b.js".parse()?].functions[0];
    assert_eq!(module.udf_type, UdfType::Action);
    assert_eq!(&module.name[..], "nodeFunction");
    assert!(module.pos.is_none());

    let application = Application::new_for_tests(&rt).await?;
    let modules = vec![ModuleConfig {
        path: "crons.js".parse()?,
        source: CRONS_SOURCE_A.to_owned(),
        source_map: None,
        environment: ModuleEnvironment::Isolate,
    }];
    let source_package = application.upload_package(&modules, None).await?;
    let result = application
        .analyze(udf_config.clone(), modules, source_package)
        .await;

    let Err(err) = result else {
        anyhow::bail!("No JsError raised for scheduled nonexistent function");
    };
    assert!(
        format!("{}", err).contains("schedules a function that does not exist"),
        "{err:?}"
    );

    let application = Application::new_for_tests(&rt).await?;
    let modules = vec![ModuleConfig {
        path: "crons.js".parse()?,
        source: CRONS_SOURCE_B.to_owned(),
        source_map: None,
        environment: ModuleEnvironment::Isolate,
    }];
    let source_package = application.upload_package(&modules, None).await?;
    let result = application
        .analyze(udf_config, modules, source_package)
        .await;

    let Err(err) = result else {
        anyhow::bail!("No JsError raised for scheduled nonexistent function");
    };
    assert!(
        format!("{}", err).contains("schedules a function that does not exist"),
        "{err:?}"
    );
    Ok(())
}
