import { invoke } from "@tauri-apps/api/core";
import type { PreferencesPort } from "../../application/ports/preferences";
import type { Preferences } from "../../domain/preferences";

export const tauriPreferences: PreferencesPort = {
  load() {
    return invoke<Preferences>("get_preferences");
  },

  save(preferences, base) {
    return invoke<Preferences>("update_preferences", { preferences, base });
  },

  applyWindowPreset(preset) {
    return invoke<void>("apply_window_preset", { preset });
  },
};
