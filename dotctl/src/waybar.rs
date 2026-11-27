use serde::Serialize;

/// The JSON contract used by Waybar's `custom/*` modules.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WaybarOutput {
    pub text: String,
    pub class: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
}

impl WaybarOutput {
    pub fn new(text: impl Into<String>, class: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            class: class.into(),
            tooltip: None,
        }
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn print(&self) {
        // Waybar parses this line. Serialising a struct of owned strings cannot fail.
        println!(
            "{}",
            serde_json::to_string(self).expect("Waybar JSON serialisation failed")
        );
    }
}
