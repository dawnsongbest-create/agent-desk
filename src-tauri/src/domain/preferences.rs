use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowBehavior {
    #[default]
    HideToTray,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub schema_version: u8,
    pub theme: ThemeMode,
    pub always_on_top: bool,
    pub window_behavior: WindowBehavior,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            theme: ThemeMode::System,
            always_on_top: false,
            window_behavior: WindowBehavior::HideToTray,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_the_frontend_wire_contract() {
        let value = serde_json::to_value(Preferences::default()).expect("serialize preferences");

        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["theme"], "system");
        assert_eq!(value["alwaysOnTop"], false);
        assert_eq!(value["windowBehavior"], "hide_to_tray");
    }
}
