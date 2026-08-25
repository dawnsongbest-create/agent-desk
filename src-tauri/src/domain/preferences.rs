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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StickyMode {
    #[default]
    Compact,
    Mini,
    Desktop,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupDisplayMode {
    #[default]
    DesktopSticky,
    LastState,
    StickyOverReader,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceDisplayMode {
    #[default]
    DesktopSticky,
    CompactTab,
    Reader,
    StickyOverReader,
    EdgeDock,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopStickyKind {
    #[default]
    Todo,
    Record,
    Quote,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderSkin {
    #[default]
    Grid,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderFontSize {
    Small,
    #[default]
    Standard,
    Large,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderLineSpacing {
    Compact,
    #[default]
    Standard,
    Relaxed,
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
pub struct DesktopStickyPosition {
    pub x: i32,
    pub y: i32,
    pub screen: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeDockPosition {
    pub y: i32,
    pub screen: Option<String>,
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
    #[serde(default)]
    pub sticky_mode: StickyMode,
    #[serde(default)]
    pub startup_display_mode: StartupDisplayMode,
    #[serde(default)]
    pub workspace_display_mode: WorkspaceDisplayMode,
    #[serde(default)]
    pub desktop_sticky_kind: DesktopStickyKind,
    #[serde(default)]
    pub desktop_sticky_position: Option<DesktopStickyPosition>,
    #[serde(default)]
    pub edge_dock_position: Option<EdgeDockPosition>,
    #[serde(default)]
    pub reader_skin: ReaderSkin,
    #[serde(default)]
    pub reader_font_size: ReaderFontSize,
    #[serde(default)]
    pub reader_line_spacing: ReaderLineSpacing,
    #[serde(default = "default_true")]
    pub reader_content_visible: bool,
    #[serde(default)]
    pub current_reader_document_id: Option<String>,
}

const fn default_true() -> bool {
    true
}

impl Preferences {
    pub const fn startup_workspace_mode(&self) -> WorkspaceDisplayMode {
        match self.startup_display_mode {
            StartupDisplayMode::DesktopSticky => WorkspaceDisplayMode::DesktopSticky,
            StartupDisplayMode::LastState => self.workspace_display_mode,
            StartupDisplayMode::StickyOverReader => WorkspaceDisplayMode::StickyOverReader,
        }
    }
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
            sticky_mode: StickyMode::Compact,
            startup_display_mode: StartupDisplayMode::DesktopSticky,
            workspace_display_mode: WorkspaceDisplayMode::DesktopSticky,
            desktop_sticky_kind: DesktopStickyKind::Todo,
            desktop_sticky_position: None,
            edge_dock_position: None,
            reader_skin: ReaderSkin::Grid,
            reader_font_size: ReaderFontSize::Standard,
            reader_line_spacing: ReaderLineSpacing::Standard,
            reader_content_visible: true,
            current_reader_document_id: None,
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
        assert_eq!(value["stickyMode"], "compact");
        assert_eq!(value["startupDisplayMode"], "desktop_sticky");
        assert_eq!(value["workspaceDisplayMode"], "desktop_sticky");
        assert_eq!(value["desktopStickyKind"], "todo");
        assert_eq!(value["desktopStickyPosition"], serde_json::Value::Null);
        assert_eq!(value["edgeDockPosition"], serde_json::Value::Null);
        assert_eq!(value["readerSkin"], "grid");
        assert_eq!(value["readerFontSize"], "standard");
        assert_eq!(value["readerLineSpacing"], "standard");
        assert_eq!(value["readerContentVisible"], true);
        assert_eq!(value["currentReaderDocumentId"], serde_json::Value::Null);
    }

    #[test]
    fn defaults_legacy_preferences_to_compact_mode() {
        let preferences: Preferences = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "theme": "light",
            "alwaysOnTop": false,
            "windowBehavior": "hide_to_tray",
            "windowPreset": "book",
            "stickyPosition": { "xRatio": 0.7, "yRatio": 0.4, "snap": null }
        }))
        .expect("deserialize legacy preferences");

        assert_eq!(preferences.sticky_mode, StickyMode::Compact);
        assert_eq!(
            preferences.startup_display_mode,
            StartupDisplayMode::DesktopSticky
        );
        assert_eq!(
            preferences.workspace_display_mode,
            WorkspaceDisplayMode::DesktopSticky
        );
        assert_eq!(preferences.desktop_sticky_kind, DesktopStickyKind::Todo);
        assert_eq!(preferences.desktop_sticky_position, None);
        assert_eq!(preferences.edge_dock_position, None);
        assert_eq!(preferences.reader_skin, ReaderSkin::Grid);
        assert_eq!(preferences.reader_font_size, ReaderFontSize::Standard);
        assert_eq!(preferences.reader_line_spacing, ReaderLineSpacing::Standard);
        assert!(preferences.reader_content_visible);
        assert_eq!(preferences.current_reader_document_id, None);
    }

    #[test]
    fn round_trips_desktop_mode_kind_and_physical_monitor_position() {
        let preferences: Preferences = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "theme": "system",
            "alwaysOnTop": false,
            "windowBehavior": "hide_to_tray",
            "windowPreset": "sticky",
            "stickyPosition": { "xRatio": 0.5, "yRatio": 0.28, "snap": null },
            "stickyMode": "desktop",
            "startupDisplayMode": "last_state",
            "workspaceDisplayMode": "edge_dock",
            "desktopStickyKind": "record",
            "desktopStickyPosition": { "x": 1430, "y": 82, "screen": "DISPLAY2" },
            "edgeDockPosition": { "y": 96, "screen": "DISPLAY2" }
        }))
        .expect("deserialize desktop preferences");

        assert_eq!(preferences.sticky_mode, StickyMode::Desktop);
        assert_eq!(
            preferences.startup_display_mode,
            StartupDisplayMode::LastState
        );
        assert_eq!(
            preferences.workspace_display_mode,
            WorkspaceDisplayMode::EdgeDock
        );
        assert_eq!(preferences.desktop_sticky_kind, DesktopStickyKind::Record);
        assert_eq!(
            preferences.desktop_sticky_position,
            Some(DesktopStickyPosition {
                x: 1430,
                y: 82,
                screen: Some("DISPLAY2".to_string())
            })
        );
        assert_eq!(
            preferences.edge_dock_position,
            Some(EdgeDockPosition {
                y: 96,
                screen: Some("DISPLAY2".to_string())
            })
        );
    }

    #[test]
    fn resolves_each_startup_policy_without_losing_last_workspace_state() {
        let mut preferences = Preferences {
            workspace_display_mode: WorkspaceDisplayMode::EdgeDock,
            ..Preferences::default()
        };
        assert_eq!(
            preferences.startup_workspace_mode(),
            WorkspaceDisplayMode::DesktopSticky
        );

        preferences.startup_display_mode = StartupDisplayMode::LastState;
        assert_eq!(
            preferences.startup_workspace_mode(),
            WorkspaceDisplayMode::EdgeDock
        );

        preferences.startup_display_mode = StartupDisplayMode::StickyOverReader;
        assert_eq!(
            preferences.startup_workspace_mode(),
            WorkspaceDisplayMode::StickyOverReader
        );
    }
}
