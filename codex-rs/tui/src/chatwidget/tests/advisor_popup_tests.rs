use super::*;
use codex_features::Feature;

#[tokio::test]
async fn advisor_completion_is_visible_without_reasoning_deltas() {
    let (mut chat, mut events, _operations) = make_chatwidget_manual(/*model_override*/ None).await;
    drain_insert_history(&mut events);
    for summary in [
        "Advisor (test) reviewed in 1.0s:\nAdvice",
        "Advisor (test) failed in 1.0s: timeout",
    ] {
        chat.handle_server_notification(
            codex_app_server_protocol::ServerNotification::ItemCompleted(
                codex_app_server_protocol::ItemCompletedNotification {
                    thread_id: "thread".to_string(),
                    turn_id: "turn".to_string(),
                    completed_at_ms: 0,
                    item: codex_app_server_protocol::ThreadItem::Reasoning {
                        id: "advisor-call".to_string(),
                        summary: vec![summary.to_string()],
                        content: Vec::new(),
                    },
                },
            ),
            /*replay_kind*/ None,
        );
    }
    let rendered = drain_insert_history(&mut events)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<String>();
    insta::assert_snapshot!(rendered, @"\n• [advisor consulted]\n\n• [advisor failed]\n");
}

#[tokio::test]
async fn advisor_model_picker_shows_available_models_and_off() {
    let (mut chat, _events, _operations) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.config
        .features
        .enable(Feature::Advisor)
        .expect("enable advisor");
    chat.sync_advisor_command_enabled();
    chat.open_advisor_popup();

    insta::assert_snapshot!(render_bottom_popup(&chat, /*width*/ 80));
}
