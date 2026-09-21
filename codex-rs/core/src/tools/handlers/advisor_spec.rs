use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use std::collections::BTreeMap;

pub(crate) const ADVISOR_TOOL_NAME: &str = "advisor";

pub(crate) fn create_advisor_tool() -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: ADVISOR_TOOL_NAME.to_string(),
        description: "Consult the configured read-only advisor model. This tool takes no parameters. Call it after initial read-only orientation and before substantial work on complex or ambiguous tasks, when stuck or changing approach, and before declaring difficult work complete. The advisor receives a bounded recent transcript, cannot use tools, and returns guidance for you to evaluate before continuing."
            .to_string(),
        strict: false,
        parameters: JsonSchema::object(BTreeMap::new(), /*required*/ None, Some(false.into())),
        output_schema: None,
        defer_loading: None,
    })
}
