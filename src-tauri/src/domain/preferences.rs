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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowPreset {
    #[default]
    Sticky,
    Iphone5,
    Pocket,
    Book,
    Custom,
}

impl WindowPreset {
    pub const fn logical_size(self) -> Option<(f64, f64)> {
        match self {
            Self::Sticky => Some((320.0, 420.0)),
            Self::Iphone5 => Some((320.0, 568.0)),
            Self::Pocket => Some((360.0, 640.0)),
            Self::Book => Some((420.0, 594.0)),
            Self::Custom => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickyPosition {
    pub x_ratio: f64,
    pub y_ratio: f64,
    pub snap: Option<StickySnap>,
}

impl Eq for StickyPosition {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StickySnap {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Default for StickyPosition {
    fn default() -> Self {
        Self {
            x_ratio: 0.5,
            y_ratio: 0.28,
            snap: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub schema_version: u8,
    pub theme: ThemeMode,
    pub always_on_top: bool,
    pub window_behavior: WindowBehavior,
    #[serde(default)]
    pub window_preset: WindowPreset,
    #[serde(default)]
    pub sticky_position: StickyPosition,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            theme: ThemeMode::System,
            always_on_top: false,
            window_behavior: WindowBehavior::HideToTray,
            window_preset: WindowPreset::Sticky,
            sticky_position: StickyPosition::default(),
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
        assert_eq!(value["windowPreset"], "sticky");
        assert_eq!(value["stickyPosition"]["xRatio"], 0.5);
    }
}
