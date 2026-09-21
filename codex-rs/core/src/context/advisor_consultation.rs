use super::ContextualUserFragment;
use codex_protocol::models::ContentItemKind;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AdvisorConsultation {
    transcript: String,
}

impl AdvisorConsultation {
    pub(crate) fn new(transcript: String) -> Self {
        Self { transcript }
    }
}

impl ContextualUserFragment for AdvisorConsultation {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("advisor.consultation".to_string())
    }

    fn role(&self) -> &'static str {
        "user"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("<advisor_consultation>\n", "\n</advisor_consultation>")
    }

    fn body(&self) -> String {
        format!(
            "Review the executor transcript below. Give concise strategic guidance for the executor's next action. Do not answer the end user directly.\n\n{}",
            self.transcript
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AdvisorGuidance {
    guidance: String,
}

impl AdvisorGuidance {
    pub(crate) fn new(guidance: String) -> Self {
        Self { guidance }
    }
}

impl ContextualUserFragment for AdvisorGuidance {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("advisor.guidance".to_string())
    }

    fn role(&self) -> &'static str {
        "user"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("<advisor_guidance>\n", "\n</advisor_guidance>")
    }

    fn body(&self) -> String {
        self.guidance.clone()
    }
}
