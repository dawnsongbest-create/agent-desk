import type { Preferences, WindowPreset } from "../../domain/preferences";

export interface PreferencesPort {
  load(): Promise<Preferences>;
  save(preferences: Preferences, base?: Preferences): Promise<Preferences>;
  applyWindowPreset(preset: WindowPreset): Promise<void>;
}
