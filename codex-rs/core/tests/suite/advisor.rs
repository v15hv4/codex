use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_features::Feature;
use codex_protocol::items::TurnItem;
use codex_protocol::protocol::EventMsg;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::sse;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use core_test_support::wait_for_event_match;
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn advisor_receives_bounded_context_without_tools_and_returns_advice() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mock = responses::mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("executor-1"),
                ev_function_call(
                    "advisor-call",
                    "ask_advisor",
                    &json!({"question": "Review the plan", "draft": "Deploy after tests"})
                        .to_string(),
                ),
                ev_completed("executor-1"),
            ]),
            sse(vec![
                ev_response_created("advisor-1"),
                ev_assistant_message("advisor-message", "Check rollback before deploy."),
                ev_completed("advisor-1"),
            ]),
            sse(vec![
                ev_response_created("executor-2"),
                ev_assistant_message("final-message", "I will check rollback."),
                ev_completed("executor-2"),
            ]),
        ],
    )
    .await;
    let test = test_codex()
        .with_model("gpt-5.5")
        .with_config(|config| {
            config.advisor_model = Some("gpt-5.5".to_string());
            config
                .features
                .enable(Feature::Advisor)
                .expect("enable advisor");
            config
                .features
                .disable(Feature::CodeModeOnly)
                .expect("direct tools");
            config
                .features
                .disable(Feature::CodeMode)
                .expect("direct tools");
        })
        .build_with_auto_env(&server)
        .await?;

    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "A".repeat(16_000),
            text_elements: Vec::new(),
        }]))
        .await?;
    let advisor_item = wait_for_event_match(&test.codex, |event| match event {
        EventMsg::ItemCompleted(completed) => match &completed.item {
            TurnItem::Reasoning(item) if item.id == "advisor-advisor-call" => Some(item.clone()),
            _ => None,
        },
        _ => None,
    })
    .await;
    assert!(advisor_item.summary_text[0].contains("Check rollback before deploy."));
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = mock.requests();
    assert_eq!(requests.len(), 3);
    assert!(
        requests[0].body_json()["tools"]
            .as_array()
            .is_some_and(|tools| tools.iter().any(|tool| tool["name"] == "ask_advisor"))
    );
    let advisor_request = requests[1].body_json();
    assert_eq!(advisor_request["model"], "gpt-5.5");
    assert!(
        advisor_request["tools"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    let advisor_input = advisor_request["input"][0]["content"][0]["text"]
        .as_str()
        .expect("advisor input text");
    assert!(advisor_input.contains("Deploy after tests"));
    assert!(
        advisor_input.len() < 10_000,
        "advisor context exceeded its bound"
    );
    let output = requests[2].function_call_output("advisor-call");
    assert!(output.to_string().contains("Check rollback before deploy."));
    Ok(())
}
