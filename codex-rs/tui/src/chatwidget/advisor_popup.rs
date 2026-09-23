use super::*;

impl ChatWidget {
    pub(crate) fn open_advisor_popup(&mut self) {
        if !self.config.features.enabled(Feature::Advisor) {
            self.add_info_message(
                "Advisor mode is disabled. Enable it in /experimental and restart Codex."
                    .to_string(),
                /*hint*/ None,
            );
            return;
        }

        let current = self.config.advisor_model.clone();
        let catalog = self.model_catalog.try_list_models().unwrap_or_default();
        let mut items = self
            .config
            .advisor_models
            .iter()
            .map(|model| {
                let display_name = catalog
                    .iter()
                    .find(|preset| preset.model == *model)
                    .map(|preset| preset.display_name.clone())
                    .unwrap_or_else(|| model.clone());
                let selected_model = model.clone();
                SelectionItem {
                    name: display_name,
                    description: Some(model.clone()),
                    is_current: current.as_deref() == Some(model.as_str()),
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::UpdateAdvisorModel(Some(selected_model.clone())));
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        items.push(SelectionItem {
            name: "Off".to_string(),
            description: Some("Do not expose the advisor tool".to_string()),
            is_current: current.is_none(),
            actions: vec![Box::new(|tx| {
                tx.send(AppEvent::UpdateAdvisorModel(None));
            })],
            dismiss_on_select: true,
            ..Default::default()
        });

        self.bottom_pane.show_selection_view(SelectionViewParams {
            items,
            header: self.model_menu_header(
                "Select Advisor",
                "The executor can consult this read-only model. Jev needs a TypeSafe API key.",
            ),
            ..SelectionViewParams::picker()
        });
    }
}
