use super::*;
use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use pretty_assertions::assert_eq;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::body_json;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[tokio::test]
async fn jev_key_file_is_used_when_environment_key_is_missing() {
    let home = tempfile::tempdir().unwrap();
    let key_path = home.path().join(JEV_KEY_FILE);
    std::fs::create_dir_all(key_path.parent().unwrap()).unwrap();
    std::fs::write(&key_path, "file-key\n").unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).unwrap();

    assert_eq!(
        jev_api_key(home.path(), None).await,
        Ok("file-key".to_string())
    );
    assert_eq!(
        jev_api_key(home.path(), Some("env-key".to_string())).await,
        Ok("env-key".to_string())
    );
}

#[cfg(unix)]
#[tokio::test]
async fn jev_key_file_rejects_access_by_other_users() {
    let home = tempfile::tempdir().unwrap();
    let key_path = home.path().join(JEV_KEY_FILE);
    std::fs::create_dir_all(key_path.parent().unwrap()).unwrap();
    std::fs::write(&key_path, "file-key\n").unwrap();
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644)).unwrap();

    assert_eq!(
        jev_api_key(home.path(), None).await,
        Err("Jev key file must be private (chmod 600)".to_string())
    );
}

#[tokio::test]
async fn jev_request_uses_typed_question_and_returns_probability() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .and(header("Authorization", "Bearer test-key"))
        .and(body_json(json!({
            "model": "jev-latest",
            "state": "recent transcript",
            "questions": {
                "judgment": {
                    "type": "noul",
                    "instructions": "Is the executor repeating an action without new information?"
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {"judgment": {"type": "noul", "noul": 0.82}}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = RouteAwareClientPool::new(
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        ClientRouteClass::Api,
    );
    let result = consult_jev(
        &client,
        &format!("{}/v1/systemone", server.uri()),
        "test-key",
        "recent transcript",
        "Is the executor repeating an action without new information?",
    )
    .await;
    assert_eq!(result, Ok(0.82));
}
