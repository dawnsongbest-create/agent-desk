export const themeModes = ["system", "light", "dark"] as const;

export type ThemeMode = (typeof themeModes)[number];
export type WindowBehavior = "hide_to_tray";

export type Preferences = {
  schemaVersion: 1;
  theme: ThemeMode;
  alwaysOnTop: boolean;
  windowBehavior: WindowBehavior;
};

export const defaultPreferences: Preferences = {
  schemaVersion: 1,
  theme: "system",
  alwaysOnTop: false,
  windowBehavior: "hide_to_tray",
};
