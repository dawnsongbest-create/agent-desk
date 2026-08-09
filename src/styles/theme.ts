import type { ThemeMode } from "../domain/preferences";

export function applyTheme(theme: ThemeMode) {
  if (theme === "system") {
    delete document.documentElement.dataset.theme;
    return;
  }

  document.documentElement.dataset.theme = theme;
}
