//! Advisor model selection for the experimental consultation tool.

use super::*;

impl ChatWidget {
    pub(crate) fn open_advisor_popup(&mut self) {
        if !self.config.features.enabled(Feature::Advisor) {
            self.add_info_message(
                "Enable Advisor in /experimental before choosing an advisor model.".to_string(),
                /*hint*/ None,
            );
            return;
        }
        let models = self.model_catalog.try_list_models().unwrap_or_default();
        let mut items = Vec::new();
        items.push(SelectionItem {
            name: "Off".to_string(),
            description: Some("Do not consult an advisor".to_string()),
            is_current: self.config.advisor_model.is_none(),
            actions: vec![Box::new(|tx| {
                tx.send(AppEvent::PersistAdvisorModel { model: None });
            })],
            ..Default::default()
        });
        for preset in models.into_iter().filter(|preset| preset.show_in_picker) {
            let model = preset.model;
            let selected = self.config.advisor_model.as_deref() == Some(model.as_str());
            items.push(SelectionItem {
                name: preset.display_name,
                description: Some(model.clone()),
                is_current: selected,
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::PersistAdvisorModel {
                        model: Some(model.clone()),
                    });
                })],
                ..Default::default()
            });
        }
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("Select advisor model".to_string()),
            subtitle: Some(
                "Codex consults it for consequential decisions and completion reviews.".to_string(),
            ),
            items,
            ..SelectionViewParams::picker()
        });
    }

    pub(crate) fn set_advisor_from_command(&mut self, requested: &str) {
        if !self.config.features.enabled(Feature::Advisor) {
            self.add_error_message("Enable Advisor in /experimental first.".to_string());
            return;
        }
        if requested.eq_ignore_ascii_case("off") || requested.eq_ignore_ascii_case("none") {
            self.app_event_tx
                .send(AppEvent::PersistAdvisorModel { model: None });
            return;
        }
        let models = self.model_catalog.try_list_models().unwrap_or_default();
        let selected = models.into_iter().find(|preset| {
            preset.model == requested || preset.display_name.eq_ignore_ascii_case(requested)
        });
        if let Some(preset) = selected {
            self.app_event_tx.send(AppEvent::PersistAdvisorModel {
                model: Some(preset.model),
            });
        } else {
            self.add_error_message(format!("Advisor model '{requested}' is unavailable."));
        }
    }

    pub(crate) fn update_advisor_model(&mut self, model: Option<String>) {
        self.config.advisor_model = model;
    }
}
