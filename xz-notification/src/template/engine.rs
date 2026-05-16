use std::collections::HashMap;

use regex::Regex;

use crate::{error::NotifError, types::{NotificationAction, PreparedNotification}};

#[derive(Debug, Default, Clone)]
pub struct TemplateEngine {
    templates: HashMap<String, String>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_template(&mut self, key: &str, template: &str) {
        self.templates.insert(key.to_string(), template.to_string());
    }

    pub fn render(
        &self,
        template_key: &str,
        vars: &HashMap<String, String>,
        _locale: Option<&str>,
    ) -> Result<PreparedNotification, NotifError> {
        let template = self
            .templates
            .get(template_key)
            .ok_or_else(|| NotifError::TemplateError(format!("template not found: {template_key}")))?;

        let re = Regex::new(r"\{\{(\w+)\}\}").expect("valid template regex");
        let rendered = re.replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            vars.get(key).cloned().unwrap_or_else(|| caps[0].to_string())
        });

        Ok(PreparedNotification {
            title: vars.get("title").cloned(),
            body: Some(rendered.into_owned()),
            subtitle: vars.get("subtitle").cloned(),
            sound: vars.get("sound").cloned(),
            actions: Vec::<NotificationAction>::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_variable_substitution() {
        let mut engine = TemplateEngine::new();
        engine.register_template("greet", "Hello {{name}}!");

        let mut vars = HashMap::new();
        vars.insert("name".into(), "World".into());

        let prepared = engine.render("greet", &vars, None).unwrap();
        assert_eq!(prepared.body.as_deref(), Some("Hello World!"));
        assert_eq!(prepared.title.as_deref(), None);
    }

    #[test]
    fn multi_variable_templates() {
        let mut engine = TemplateEngine::new();
        engine.register_template("info", "{{title}}: {{body}} / {{subtitle}} / {{sound}}");

        let mut vars = HashMap::new();
        vars.insert("title".into(), "Alert".into());
        vars.insert("body".into(), "Something happened".into());
        vars.insert("subtitle".into(), "Details".into());
        vars.insert("sound".into(), "ping".into());

        let prepared = engine.render("info", &vars, Some("en-US")).unwrap();
        assert_eq!(prepared.title.as_deref(), Some("Alert"));
        assert_eq!(prepared.body.as_deref(), Some("Alert: Something happened / Details / ping"));
        assert_eq!(prepared.subtitle.as_deref(), Some("Details"));
        assert_eq!(prepared.sound.as_deref(), Some("ping"));
    }

    #[test]
    fn missing_variable_preserves_placeholder() {
        let mut engine = TemplateEngine::new();
        engine.register_template("greet", "Hello {{name}} and {{missing}}!");

        let mut vars = HashMap::new();
        vars.insert("name".into(), "Ada".into());

        let rendered = engine.render("greet", &vars, None).unwrap();
        assert_eq!(rendered.body.as_deref(), Some("Hello Ada and {{missing}}!"));
    }

    #[test]
    fn missing_template_key_returns_error() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        let err = engine.render("missing", &vars, None).unwrap_err();
        assert_eq!(err.to_string(), "template error: template not found: missing");
    }

    #[test]
    fn locale_parameter_is_accepted() {
        let mut engine = TemplateEngine::new();
        engine.register_template("greet", "Hello {{name}}!");

        let mut vars = HashMap::new();
        vars.insert("name".into(), "World".into());

        let prepared = engine.render("greet", &vars, Some("fr-FR")).unwrap();
        assert_eq!(prepared.body.as_deref(), Some("Hello World!"));
    }
}
