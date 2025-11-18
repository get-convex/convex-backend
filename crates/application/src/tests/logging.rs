use std::{
    assert_matches::assert_matches,
    time::Duration,
};

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
        PublicFunctionPath,
    },
    knobs,
    log_streaming::StructuredLogEvent,
    runtime::{
        testing::TestRuntime,
        Runtime,
    },
    types::{
        FunctionCaller,
        UdfType,
    },
    version::ClientVersion,
    RequestId,
};
use keybroker::Identity;
use log_streaming::sinks::mock_sink::MOCK_SINK_EVENTS_BUFFER;
use model::log_sinks::{
    types::{
        LogSinksRow,
        SinkConfig,
        SinkState,
        SinkType,
    },
    LogSinksModel,
};
use must_let::must_let;
use serde_json::json;

use crate::{
    test_helpers::ApplicationTestExt,
    Application,
};

#[convex_macro::test_runtime]
async fn test_udf_logs(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    // Note that this loads CRONs which we should unit tests for as well
    application.load_udf_tests_modules().await?;

    // Create Mock sink
    let mut tx = application.begin(Identity::system()).await?;
    let mut model = LogSinksModel::new(&mut tx);
    model.add_or_update(SinkConfig::Mock).await?;
    application.commit_test(tx).await?;

    rt.wait(Duration::from_secs(1)).await;

    // Assert Mock sink exists
    let mut tx = application.begin(Identity::system()).await?;
    let mut model = LogSinksModel::new(&mut tx);
    let res = model
        .get_by_provider(SinkType::Mock)
        .await?
        .map(|d| d.into_value());

    assert_matches!(
        res,
        Some(LogSinksRow {
            status: SinkState::Active,
            ..
        })
    );

    // Read Mock
    let path = CanonicalizedComponentFunctionPath {
        component: ComponentPath::test_user(),
        udf_path: "logging:logString".parse()?,
    };
    let result = application
        .read_only_udf(
            RequestId::new(),
            PublicFunctionPath::Component(path),
            vec![json!({})],
            Identity::system(),
            FunctionCaller::SyncWorker(ClientVersion::unknown()),
        )
        .await?;
    assert!(result.result.is_ok());

    rt.wait(Duration::from_millis(
        *knobs::LOG_MANAGER_AGGREGATION_INTERVAL_MILLIS,
    ))
    .await;

    let buf = MOCK_SINK_EVENTS_BUFFER
        .read()
        .iter()
        .map(|e| e.event.clone())
        .collect::<Vec<_>>();
    tracing::info!("buf: {buf:#?}");

    must_let!(let StructuredLogEvent::Console { log_line, .. } = &buf[0]);
    assert_eq!(log_line.clone().to_pretty_string(), "[LOG] 'myString'");

    must_let!(let StructuredLogEvent::FunctionExecution { source, .. } = &buf[1]);
    assert_eq!(source.udf_path, "logging:logString");
    assert_eq!(source.udf_type, UdfType::Query);
    assert_eq!(source.cached, Some(false));

    assert_eq!(buf.len(), 2);

    Ok(())
}
