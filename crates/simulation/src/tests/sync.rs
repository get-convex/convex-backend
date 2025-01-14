use common::{
    assert_obj,
    obj,
    value::{
        array,
        ConvexArray,
        ConvexObject,
        ConvexValue,
    },
};
use must_let::must_let;
use runtime::testing::TestRuntime;

use crate::test_helpers::{
    js_client::MutationInfo,
    simulation::{
        SimulationTest,
        SimulationTestConfig,
    },
};

fn _assert_results_match(expected: &ConvexValue, actual: &ConvexValue) -> anyhow::Result<()> {
    if let ConvexValue::Object(actual_object) = actual {
        if let ConvexValue::Object(expected_object) = expected {
            return assert_objects_match(expected_object, actual_object);
        } else {
            anyhow::bail!("Expected and actual are different types");
        }
    }

    if let ConvexValue::Array(actual_array) = actual {
        if let ConvexValue::Array(expected_array) = expected {
            return assert_arrays_match(expected_array, actual_array);
        } else {
            anyhow::bail!("Expected and actual are different types");
        }
    }

    if expected != actual {
        anyhow::bail!("Expected {:#}, got {:#}", expected, actual);
    }
    Ok(())
}

fn assert_results_match(expected: &ConvexValue, actual: &ConvexValue) -> anyhow::Result<()> {
    if _assert_results_match(expected, actual).is_err() {
        let expected_json = serde_json::to_value(expected)?;
        let actual_json = serde_json::to_value(actual)?;
        anyhow::bail!(
            "Expected {:#}, got {:#}",
            serde_json::to_string_pretty(&expected_json)?,
            serde_json::to_string_pretty(&actual_json)?
        );
    }
    Ok(())
}

fn assert_objects_match(expected: &ConvexObject, actual: &ConvexObject) -> anyhow::Result<()> {
    for (key, value) in expected.iter() {
        let actual_value = actual
            .get(key)
            .ok_or(anyhow::anyhow!("Key not found: {}", key))?;
        _assert_results_match(value, actual_value)?;
    }
    Ok(())
}

fn assert_arrays_match(expected: &ConvexArray, actual: &ConvexArray) -> anyhow::Result<()> {
    for (expected_value, actual_value) in expected.iter().zip(actual.iter()) {
        _assert_results_match(expected_value, actual_value)?;
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_sync(rt: TestRuntime) -> anyhow::Result<()> {
    SimulationTest::run(
        rt.clone(),
        SimulationTestConfig {
            num_client_threads: 1,
            expected_delay_duration: None,
        },
        async |t: SimulationTest| {
            t.server
                .mutation("misc:init".parse()?, assert_obj!())
                .await??;

            let client = &t.js_clients[0];
            let subscription_id = client
                .add_sync_query("getConversations", assert_obj!())
                .await?;

            let result = client.sync_query_result(subscription_id.clone()).await?;
            assert_eq!(result, None);

            let ts = t.server.latest_timestamp().await?;
            t.js_clients[0].wait_for_server_ts(ts).await?;

            let result = client.sync_query_result(subscription_id.clone()).await?;

            assert_eq!(result, Some(Ok(ConvexValue::Array(array![]))));

            client.disconnect_network().await?;
            let mutation_id_a = client
                .request_sync_mutation(MutationInfo {
                    mutation_path: "conversations:create".parse()?,
                    opt_update_args: obj!("emoji" => "a", "id" => "a")?,
                    server_args: obj!("emoji" => "a",)?,
                })
                .await?;
            let mutation_id_b = client
                .request_sync_mutation(MutationInfo {
                    mutation_path: "conversations:create".parse()?,
                    opt_update_args: obj!("emoji" => "b", "id" => "b")?,
                    server_args: obj!("emoji" => "b")?,
                })
                .await?;
            client
                .wait_for_sync_mutation_reflected_locally(mutation_id_a)
                .await?;
            client
                .wait_for_sync_mutation_reflected_locally(mutation_id_b)
                .await?;
            let result = client.sync_query_result(subscription_id.clone()).await?;

            must_let!(let Some(Ok(result)) = result);
            assert_results_match(
                &ConvexValue::Array(array![
                    obj!("emoji" => "b")?.into(),
                    obj!("emoji" => "a")?.into(),
                ]?),
                &result,
            )?;

            client.remove_sync_query(subscription_id.clone()).await?;

            Ok(())
        },
    )
    .await
}

#[ignore] // Test disabled due to ENG-8227
#[convex_macro::test_runtime]
async fn test_new_sync_query_after_disconnect(rt: TestRuntime) -> anyhow::Result<()> {
    SimulationTest::run(
        rt.clone(),
        SimulationTestConfig {
            num_client_threads: 1,
            expected_delay_duration: None,
        },
        async |t: SimulationTest| {
            t.server
                .mutation("misc:init".parse()?, assert_obj!())
                .await??;
            t.server
                .mutation("conversations:create".parse()?, assert_obj!("emoji" => "a"))
                .await??;

            let client = &t.js_clients[0];
            let subscription_id = client
                .add_sync_query("getConversations", assert_obj!())
                .await?;
            let ts = t.server.latest_timestamp().await?;
            client.wait_for_server_ts(ts).await?;

            client.disconnect_network().await?;

            let result = client.sync_query_result(subscription_id.clone()).await?;
            must_let!(let Some(Ok(ConvexValue::Array(conversations))) = result);
            must_let!(let ConvexValue::Object(conversation) = conversations[0].clone());
            must_let!(let ConvexValue::String(conversation_id) = conversation.get("_id").unwrap());
            let conversation_id = conversation_id.to_string();

            let offline_subscription_id = client
                .add_sync_query(
                    "getSingleConversation",
                    assert_obj!("conversationId" => conversation_id),
                )
                .await?;
            let offline_result = client
                .sync_query_result(offline_subscription_id.clone())
                .await?;
            must_let!(let Some(Ok(ConvexValue::Object(offline_conversation))) = offline_result);
            assert_eq!(offline_conversation, conversation);

            Ok(())
        },
    )
    .await
}
