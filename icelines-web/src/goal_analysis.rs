//! Shared HTML surface for native routes and portable CLI reports.
use anyhow::Result;
use askama::Template;
use serde_json::Value;
#[derive(Clone)]
pub struct GoalChoice {
    pub target: String,
    pub label: String,
    pub available: bool,
}
#[derive(Template)]
#[template(path = "goal_analysis.html")]
pub(crate) struct Page {
    pub active_label: String,
    pub data_json: String,
    pub target: String,
    pub message: String,
    pub choices: Vec<GoalChoice>,
    pub standalone: bool,
}
#[derive(Template)]
#[template(path = "goal_analysis_export.html")]
struct Export {
    data_json: String,
    target: String,
    message: String,
    choices: Vec<GoalChoice>,
    standalone: bool,
}
pub fn script_json(data: &Value) -> Result<String> {
    Ok(serde_json::to_string(data)?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026"))
}
pub fn render_goal_analysis_report(data: &Value) -> Result<String> {
    anyhow::ensure!(
        data["schema"] == "icelines.goal-analysis.v0.1",
        "Unsupported goal analysis schema"
    );
    Ok(Export {
        data_json: script_json(data)?,
        target: String::new(),
        message: String::new(),
        choices: vec![],
        standalone: true,
    }
    .render()?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn script_payload_cannot_close_its_element() {
        let s = script_json(&serde_json::json!({"name":"</script><img src=x onerror=alert(1)>"}))
            .unwrap();
        assert!(!s.contains('<'));
        assert_eq!(
            serde_json::from_str::<Value>(&s).unwrap()["name"],
            "</script><img src=x onerror=alert(1)>"
        );
    }
}
