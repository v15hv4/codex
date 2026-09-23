use anyhow::Result;
use codex_features::Feature;
use core_test_support::responses;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn advisor_tool_consults_selected_model_and_returns_guidance() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.features.enable(Feature::Advisor).unwrap();
        config.advisor_model = Some("gpt-6-astra".to_string());
        config.advisor_models = vec!["gpt-6-astra".to_string(), "gpt-5.6-sol".to_string()];
    });
    let test = builder.build_with_auto_env(&server).await?;
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_function_call("advisor-call", "advisor", "{}"),
                ev_completed("executor-before-advice"),
            ]),
            sse(vec![
                ev_assistant_message("advisor-guidance", "Check the retry boundary first."),
                ev_completed("advisor-response"),
            ]),
            sse(vec![
                ev_assistant_message("executor-final", "I checked the retry boundary."),
                ev_completed("executor-after-advice"),
            ]),
        ],
    )
    .await;

    test.submit_turn("Diagnose the retry bug").await?;

    let requests = mock.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].body_json()["model"], "gpt-6-astra");
    let advisor_input = serde_json::to_string(&requests[1].input())?;
    assert!(advisor_input.contains("Diagnose the retry bug"));
    assert!(advisor_input.contains("advisor_consultation"));
    let guidance = requests[2].function_call_output("advisor-call");
    let guidance = guidance["output"]
        .as_str()
        .expect("advisor guidance should be text");
    assert!(guidance.contains("<advisor_guidance>"));
    assert!(guidance.contains("Check the retry boundary first."));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn advisor_tool_is_absent_without_a_selected_model() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.features.enable(Feature::Advisor).unwrap();
        config.advisor_model = None;
    });
    let test = builder.build_with_auto_env(&server).await?;
    let mock = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("done", "No advisor selected."),
            ev_completed("done"),
        ]),
    )
    .await;

    test.submit_turn("Hello").await?;

    let tools = mock.single_request().body_json()["tools"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(!tools.iter().any(|tool| tool["name"] == "advisor"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn jev_advisor_exposes_a_required_question() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.features.enable(Feature::Advisor).unwrap();
        config.advisor_model = Some("jev-latest".to_string());
        config.advisor_models = vec!["jev-latest".to_string()];
    });
    let test = builder.build_with_auto_env(&server).await?;
    let mock = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("done", "Done."),
            ev_completed("done"),
        ]),
    )
    .await;

    test.submit_turn("Review the current approach").await?;

    let tools = mock.single_request().body_json()["tools"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let advisor = tools.iter().find(|tool| tool["name"] == "advisor").unwrap();
    assert_eq!(advisor["parameters"]["required"], json!(["question"]));
    assert_eq!(
        advisor["parameters"]["properties"]["question"]["type"],
        "string"
    );
    Ok(())
}
