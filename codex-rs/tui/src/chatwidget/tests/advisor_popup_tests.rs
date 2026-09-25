use super::*;
use codex_features::Feature;

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
