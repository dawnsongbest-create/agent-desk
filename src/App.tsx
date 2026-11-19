import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Preferences } from "./domain/preferences";
import { defaultPreferences } from "./domain/preferences";
import { StickyHome } from "./features/sticky/StickyHome";
import { tauriPreferences } from "./infrastructure/tauri/preferences";
import { tauriDeliveries } from "./infrastructure/tauri/delivery";
import { tauriReaderDocuments } from "./infrastructure/tauri/reader";
import { tauriReadingPlans } from "./infrastructure/tauri/reading";
import { tauriAgentProposals } from "./infrastructure/tauri/proposal";
import { tauriAgentConnection } from "./infrastructure/tauri/agentConnection";
import { tauriStickyCards } from "./infrastructure/tauri/sticky";
import { tauriDesktopSticky } from "./infrastructure/tauri/desktopSticky";
import { applyTheme } from "./styles/theme";
import "./App.css";

type SaveState = "loading" | "idle" | "saving" | "saved" | "error";

type AppProps = { surface?: "main" | "desktop" };

function App({ surface = "main" }: AppProps) {
  const [preferences, setPreferences] = useState<Preferences>(defaultPreferences);
  const [saveState, setSaveState] = useState<SaveState>("loading");
  const preferencesRef = useRef(defaultPreferences);
  const latestSave = useRef(0);
  const saveQueue = useRef<Promise<void>>(Promise.resolve());

  useEffect(() => {
    let cancelled = false;

    tauriPreferences
      .load()
      .then((stored) => {
        if (cancelled) return;
        preferencesRef.current = stored;
        setPreferences(stored);
        applyTheme(stored.theme);
        setSaveState("idle");
      })
      .catch(() => {
        if (!cancelled) setSaveState("error");
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    let dispose: () => void = () => undefined;
    void listen<Preferences>("agent-desk://preferences-changed", ({ payload }) => {
      preferencesRef.current = payload;
      setPreferences(payload);
      applyTheme(payload.theme);
      setSaveState("idle");
    })
      .then((unlisten) => {
        if (cancelled) unlisten();
        else dispose = unlisten;
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      dispose();
    };
  }, []);

  function save(next: Preferences) {
    const base = preferencesRef.current;
    preferencesRef.current = next;
    setPreferences(next);
    applyTheme(next.theme);
    setSaveState("saving");
    const saveSequence = ++latestSave.current;

    saveQueue.current = saveQueue.current
      .catch(() => undefined)
      .then(() => tauriPreferences.save(next, base))
      .then((stored) => {
        if (saveSequence !== latestSave.current) return;
        preferencesRef.current = stored;
        setPreferences(stored);
        applyTheme(stored.theme);
        setSaveState("saved");
      })
      .catch(() => {
        if (saveSequence === latestSave.current) setSaveState("error");
      });
  }

  return (
    <StickyHome
      port={tauriStickyCards}
      readerPort={tauriReaderDocuments}
      deliveryPort={tauriDeliveries}
      readingPort={tauriReadingPlans}
      proposalPort={tauriAgentProposals}
      agentConnectionPort={tauriAgentConnection}
      preferences={preferences}
      nativeSurface={surface}
      preferenceSaveState={saveState}
      onThemeChange={(theme) => save({ ...preferencesRef.current, theme })}
      onAlwaysOnTopChange={(alwaysOnTop) => save({ ...preferencesRef.current, alwaysOnTop })}
      onWindowPresetChange={(windowPreset) => {
        void tauriPreferences
          .applyWindowPreset(windowPreset)
          .then(() => save({ ...preferencesRef.current, windowPreset }))
          .catch(() => setSaveState("error"));
      }}
      onStickyPositionChange={(stickyPosition) =>
        save({ ...preferencesRef.current, stickyPosition })
      }
      onStickyModeChange={(stickyMode) => {
        setSaveState("saving");
        const operation =
          surface === "desktop"
            ? tauriDesktopSticky.setDesktopMode(stickyMode)
            : tauriDesktopSticky.setReaderMode(stickyMode);
        void operation
          .then((stored) => {
            preferencesRef.current = stored;
            setPreferences(stored);
            setSaveState("saved");
          })
          .catch(() => setSaveState("error"));
      }}
      onStartupDisplayModeChange={(startupDisplayMode) =>
        save({ ...preferencesRef.current, startupDisplayMode })
      }
      onOpenReaderWorkspace={() => {
        setSaveState("saving");
        void tauriDesktopSticky
          .openReader()
          .then((stored) => {
            preferencesRef.current = stored;
            setPreferences(stored);
            setSaveState("saved");
          })
          .catch(() => setSaveState("error"));
      }}
      onLeaveReaderWorkspace={(stickyMode) => {
        setSaveState("saving");
        void tauriDesktopSticky
          .leaveReader(stickyMode)
          .then((stored) => {
            preferencesRef.current = stored;
            setPreferences(stored);
            setSaveState("saved");
          })
          .catch(() => setSaveState("error"));
      }}
      onReaderFontSizeChange={(readerFontSize) =>
        save({ ...preferencesRef.current, readerFontSize })
      }
      onReaderLineSpacingChange={(readerLineSpacing) =>
        save({ ...preferencesRef.current, readerLineSpacing })
      }
      onReaderContentVisibilityChange={(readerContentVisible) =>
        save({ ...preferencesRef.current, readerContentVisible })
      }
      onCurrentReaderDocumentChange={(currentReaderDocumentId) =>
        save({ ...preferencesRef.current, currentReaderDocumentId })
      }
      onReaderScrollTopChange={(readerScrollTop) =>
        save({ ...preferencesRef.current, readerScrollTop })
      }
      onReaderSurfaceModeChange={(readerSurfaceMode) =>
        save({ ...preferencesRef.current, readerSurfaceMode })
      }
    />
  );
}

export default App;
