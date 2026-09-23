use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use std::collections::BTreeMap;

pub(crate) const ADVISOR_TOOL_NAME: &str = "advisor";

pub(crate) fn create_advisor_tool(model: &str) -> ToolSpec {
    let is_jev = model == "jev-latest";
    let parameters = if is_jev {
        JsonSchema::object(
            BTreeMap::from([(
                "question".to_string(),
                JsonSchema::string(Some(
                    "One focused yes/no question about the recent transcript. Ask for a judgment, not generated advice."
                        .to_string(),
                )),
            )]),
            Some(vec!["question".to_string()]),
            Some(false.into()),
        )
    } else {
        JsonSchema::object(BTreeMap::new(), /*required*/ None, Some(false.into()))
    };
    ToolSpec::Function(ResponsesApiTool {
        name: ADVISOR_TOOL_NAME.to_string(),
        description: if is_jev {
            "Ask Jev one focused yes/no question about the recent transcript. Jev returns a probability, not text or a plan. Use the result as one piece of evidence when deciding what to do next. The question must be answerable from the transcript."
        } else {
            "Consult the configured read-only advisor model. This tool takes no parameters. Call it after initial read-only orientation and before substantial work on complex or ambiguous tasks, when stuck or changing approach, and before declaring difficult work complete. The advisor receives a bounded recent transcript, cannot use tools, and returns guidance for you to evaluate before continuing."
        }.to_string(),
        strict: false,
        parameters,
        output_schema: None,
        defer_loading: None,
    })
}
