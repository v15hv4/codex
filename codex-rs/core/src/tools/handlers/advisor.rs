use crate::Prompt;
use crate::client_common::ResponseEvent;
use crate::context::AdvisorConsultation;
use crate::context::AdvisorGuidance;
use crate::context::ContextualUserFragment;
use crate::function_tool::FunctionCallError;
use crate::responses_metadata::CodexResponsesRequestKind;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::advisor_spec::ADVISOR_TOOL_NAME;
use crate::tools::handlers::advisor_spec::create_advisor_tool;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::BaseInstructionsProvenance;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TruncationPolicy;
use codex_rollout_trace::InferenceTraceContext;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use codex_utils_output_truncation::approx_token_count;
use codex_utils_output_truncation::truncate_text;
use futures::StreamExt;

const ADVISOR_TRANSCRIPT_MAX_TOKENS: usize = 8_000;
const ADVISOR_GUIDANCE_MAX_TOKENS: usize = 2_000;
const ADVISOR_BASE_INSTRUCTIONS: &str = "You are a read-only advisor to another coding agent. Analyze the supplied executor transcript. Identify important constraints, risks, incorrect assumptions, and the best next actions. Do not call tools. Do not address the end user. Return only concise guidance to the executor.";

pub struct AdvisorHandler {
    model: String,
}

impl AdvisorHandler {
    pub(crate) fn new(model: String) -> Self {
        Self { model }
    }
}

impl ToolExecutor<ToolInvocation> for AdvisorHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain(ADVISOR_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        create_advisor_tool()
    }

    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        Box::pin(async move {
            let ToolPayload::Function { arguments } = &invocation.payload else {
                return Err(FunctionCallError::RespondToModel(
                    "advisor handler received unsupported payload".to_string(),
                ));
            };
            if !arguments.trim().is_empty() && arguments.trim() != "{}" {
                return Err(FunctionCallError::RespondToModel(
                    "advisor takes no parameters".to_string(),
                ));
            }

            let model_info = invocation
                .session
                .services
                .models_manager
                .get_model_info(
                    &self.model,
                    &invocation.turn.config.to_models_manager_config(),
                )
                .await;
            let transcript = bounded_transcript(
                invocation.session.clone_history().await.raw_items(),
                ADVISOR_TRANSCRIPT_MAX_TOKENS,
            );
            let prompt = Prompt {
                input: vec![ContextualUserFragment::into(AdvisorConsultation::new(
                    transcript,
                ))],
                base_instructions: BaseInstructions {
                    text: ADVISOR_BASE_INSTRUCTIONS.to_string(),
                    provenance: Some(BaseInstructionsProvenance::Custom),
                },
                ..Default::default()
            };
            let metadata = invocation
                .session
                .responses_metadata(&invocation.step_context, CodexResponsesRequestKind::Advisor)
                .await;
            let mut client_session = invocation.session.services.model_client.new_session();
            let mut stream = client_session
                .stream(
                    &prompt,
                    &model_info,
                    &invocation.step_context.session_telemetry,
                    model_info.default_reasoning_level.clone(),
                    model_info.default_reasoning_summary,
                    /*service_tier*/ None,
                    &metadata,
                    &InferenceTraceContext::disabled(),
                )
                .await
                .map_err(|err| {
                    FunctionCallError::RespondToModel(format!(
                        "advisor model `{}` could not start: {err}",
                        self.model
                    ))
                })?;

            let mut output = Vec::new();
            loop {
                let event = tokio::select! {
                    _ = invocation.cancellation_token.cancelled() => {
                        return Err(FunctionCallError::RespondToModel(
                            "advisor consultation was cancelled".to_string(),
                        ));
                    }
                    event = stream.next() => event,
                };
                let Some(event) = event else {
                    return Err(FunctionCallError::RespondToModel(
                        "advisor response ended before completion".to_string(),
                    ));
                };
                match event {
                    Ok(ResponseEvent::OutputItemDone(item)) => output.push(item),
                    Ok(ResponseEvent::Completed { token_usage, .. }) => {
                        invocation
                            .session
                            .update_token_usage_info(&invocation.turn, token_usage.as_ref())
                            .await
                            .map_err(|err| {
                                FunctionCallError::RespondToModel(format!(
                                    "advisor usage could not be recorded: {err}"
                                ))
                            })?;
                        break;
                    }
                    Ok(_) => {}
                    Err(err) => {
                        return Err(FunctionCallError::RespondToModel(format!(
                            "advisor model `{}` failed: {err}",
                            self.model
                        )));
                    }
                }
            }

            let guidance = last_assistant_text(&output).ok_or_else(|| {
                FunctionCallError::RespondToModel(format!(
                    "advisor model `{}` returned no guidance",
                    self.model
                ))
            })?;
            let guidance = truncate_text(
                &guidance,
                TruncationPolicy::Tokens(ADVISOR_GUIDANCE_MAX_TOKENS),
            );
            Ok(boxed_tool_output(FunctionToolOutput::from_text(
                AdvisorGuidance::new(guidance).render(),
                Some(true),
            )))
        })
    }
}

impl CoreToolRuntime for AdvisorHandler {}

fn bounded_transcript<'a>(
    items: impl DoubleEndedIterator<Item = &'a ResponseItem>,
    max_tokens: usize,
) -> String {
    let mut remaining = max_tokens;
    let mut lines = Vec::new();
    for item in items.rev() {
        let Ok(serialized) = serde_json::to_string(item) else {
            continue;
        };
        let tokens = approx_token_count(&serialized);
        if tokens > remaining {
            if lines.is_empty() {
                lines.push(truncate_text(
                    &serialized,
                    TruncationPolicy::Tokens(remaining),
                ));
            }
            break;
        }
        remaining = remaining.saturating_sub(tokens);
        lines.push(serialized);
    }
    lines.reverse();
    lines.join("\n")
}

fn last_assistant_text(items: &[ResponseItem]) -> Option<String> {
    items.iter().rev().find_map(|item| {
        let ResponseItem::Message { role, content, .. } = item else {
            return None;
        };
        (role == "assistant")
            .then(|| crate::content_items_to_text(content))
            .flatten()
            .filter(|text| !text.trim().is_empty())
    })
}
