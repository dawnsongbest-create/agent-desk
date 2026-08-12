import type { Preferences, WindowPreset } from "../../domain/preferences";

export interface PreferencesPort {
  load(): Promise<Preferences>;
  save(preferences: Preferences): Promise<Preferences>;
  applyWindowPreset(preset: WindowPreset): Promise<void>;
}
