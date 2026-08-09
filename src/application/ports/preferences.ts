import type { Preferences } from "../../domain/preferences";

export interface PreferencesPort {
  load(): Promise<Preferences>;
  save(preferences: Preferences): Promise<Preferences>;
}
