import type { StickyCardsPort } from "../../application/ports/sticky";
import type { ReaderDocumentsPort } from "../../application/ports/reader";
import type { DeliveriesPort } from "../../application/ports/delivery";
import type { ReadingPlansPort } from "../../application/ports/reading";
import type { AgentProposalsPort } from "../../application/ports/proposal";
import type { TodoSuggestionsPort } from "../../application/ports/todoSuggestions";
import type { AgentConnectionPort } from "../../application/ports/agentConnection";
import type {
  Preferences,
  ReaderFontSize,
  ReaderLineSpacing,
  StartupDisplayMode,
  StickyMode,
  StickyPosition,
  ThemeMode,
  WindowPreset,
} from "../../domain/preferences";
import { StickyShell } from "./StickyShell";
import { useStickyCards } from "./useStickyCards";
import { useReaderDocument } from "../reader/useReaderDocument";
import { useInbox } from "../inbox/useInbox";
import { useAgentProposals } from "../proposal/useAgentProposals";
import { useAgentBridge } from "../agentBridge/useAgentBridge";

type StickyHomeProps = {
  nativeSurface?: "main" | "desktop";
  port: StickyCardsPort;
  readerPort: ReaderDocumentsPort;
  deliveryPort: DeliveriesPort;
  readingPort: ReadingPlansPort;
  proposalPort: AgentProposalsPort;
  todoSuggestionsPort?: TodoSuggestionsPort;
  agentConnectionPort: AgentConnectionPort;
  preferences: Preferences;
  preferenceSaveState: "loading" | "idle" | "saving" | "saved" | "error";
  now?: Date;
  onThemeChange(theme: ThemeMode): void;
  onAlwaysOnTopChange(alwaysOnTop: boolean): void;
  onWindowPresetChange(windowPreset: WindowPreset): void;
  onStickyPositionChange(position: StickyPosition): void;
  onStickyModeChange(mode: StickyMode): void;
  onStartupDisplayModeChange(mode: StartupDisplayMode): void;
  onOpenReaderWorkspace(): void;
  onLeaveReaderWorkspace(mode: StickyMode): void;
  onReaderFontSizeChange(size: ReaderFontSize): void;
  onReaderLineSpacingChange(spacing: ReaderLineSpacing): void;
  onReaderContentVisibilityChange(visible: boolean): void;
  onCurrentReaderDocumentChange(id: string): void;
  onReaderScrollTopChange(scrollTop: number): void;
  onReaderSurfaceModeChange(mode: Preferences["readerSurfaceMode"]): void;
};

export function StickyHome({
  nativeSurface = "main",
  port,
  readerPort,
  deliveryPort,
  readingPort,
  proposalPort,
  todoSuggestionsPort,
  agentConnectionPort,
  preferences,
  preferenceSaveState,
  now,
  onThemeChange,
  onAlwaysOnTopChange,
  onWindowPresetChange,
  onStickyPositionChange,
  onStickyModeChange,
  onStartupDisplayModeChange,
  onOpenReaderWorkspace,
  onLeaveReaderWorkspace,
  onReaderFontSizeChange,
  onReaderLineSpacingChange,
  onReaderContentVisibilityChange,
  onCurrentReaderDocumentChange,
  onReaderScrollTopChange,
  onReaderSurfaceModeChange,
}: StickyHomeProps) {
  const sticky = useStickyCards(port);
  const reader = useReaderDocument(
    readerPort,
    preferenceSaveState !== "loading",
    preferences.currentReaderDocumentId,
    onCurrentReaderDocumentChange,
  );
  const inbox = useInbox(deliveryPort, preferenceSaveState !== "loading");
  const proposals = useAgentProposals(proposalPort, reader.document?.id ?? null);
  const agentBridge = useAgentBridge(agentConnectionPort);

  async function captureSelection(documentId: string, text: string) {
    try {
      const captured = await reader.captureSelection(documentId, text);
      sticky.acceptCreated(captured.record);
      return true;
    } catch {
      return false;
    }
  }

  async function openDelivery(id: string) {
    const opened = await inbox.open(id);
    if (!opened) return false;
    reader.acceptOpened(opened.document);
    onCurrentReaderDocumentChange(opened.document.id);
    onReaderContentVisibilityChange(true);
    return true;
  }

  async function createReadingSession(documentId: string, text: string) {
    try {
      const created = await readingPort.createSession({
        sourceDocumentId: documentId,
        content: text,
      });
      reader.acceptOpened(created.document);
      onCurrentReaderDocumentChange(created.document.id);
      onReaderContentVisibilityChange(true);
      return true;
    } catch {
      return false;
    }
  }

  async function acceptProposal(id: string) {
    const accepted = await proposals.accept(id);
    if (!accepted) return false;
    if (accepted.card) sticky.acceptCreated(accepted.card);
    if (accepted.readingSession) {
      reader.acceptOpened(accepted.readingSession.document);
      onCurrentReaderDocumentChange(accepted.readingSession.document.id);
      onReaderContentVisibilityChange(true);
    }
    return true;
  }

  return (
    <StickyShell
      preferences={preferences}
      nativeSurface={nativeSurface}
      readerDocument={reader.document}
      readerState={reader.state}
      inboxItems={inbox.items}
      inboxUnreadCount={inbox.unreadCount}
      inboxState={inbox.state}
      inboxOpeningId={inbox.openingId}
      inboxOpenError={inbox.openError}
      agentProposals={proposals.proposals}
      proposalBusyId={proposals.busyId}
      proposalErrorId={proposals.errorId}
      todoSuggestionsPort={todoSuggestionsPort}
      onTodoBatchAccepted={sticky.refreshTodoCards}
      agentBridgeStatus={agentBridge.status}
      agentBridgeState={agentBridge.state}
      issuedAgentToken={agentBridge.issuedToken}
      agentTokenCopied={agentBridge.copied}
      preferenceSaveState={preferenceSaveState}
      cards={sticky.cards}
      profile={sticky.profile}
      stickyState={sticky.state}
      error={sticky.error}
      now={now}
      onThemeChange={onThemeChange}
      onAlwaysOnTopChange={onAlwaysOnTopChange}
      onWindowPresetChange={onWindowPresetChange}
      onStickyPositionChange={onStickyPositionChange}
      onStickyModeChange={onStickyModeChange}
      onStartupDisplayModeChange={onStartupDisplayModeChange}
      onOpenReaderWorkspace={onOpenReaderWorkspace}
      onLeaveReaderWorkspace={onLeaveReaderWorkspace}
      onReaderFontSizeChange={onReaderFontSizeChange}
      onReaderLineSpacingChange={onReaderLineSpacingChange}
      onReaderContentVisibilityChange={onReaderContentVisibilityChange}
      onReaderScrollTopChange={onReaderScrollTopChange}
      onReaderSurfaceModeChange={onReaderSurfaceModeChange}
      onRetryReader={reader.retry}
      onRetryInbox={inbox.retry}
      onOpenDelivery={openDelivery}
      onCopyReaderSelection={reader.copyText}
      onCaptureReaderSelection={captureSelection}
      onCreateReadingSession={createReadingSession}
      onAcceptProposal={acceptProposal}
      onRejectProposal={proposals.reject}
      onAgentBridgeEnabledChange={agentBridge.setEnabled}
      onGenerateAgentToken={agentBridge.generateToken}
      onCopyAgentToken={agentBridge.copyToken}
      onRetryAgentBridge={agentBridge.refresh}
      onCreate={sticky.create}
      onUpdateText={sticky.updateText}
      onTaskCompleted={sticky.setTaskCompleted}
      onTaskDueDate={sticky.setTaskDueDate}
      onDelete={sticky.deleteCard}
      onReorder={sticky.reorder}
      onUpdateQuote={sticky.updateQuote}
      onExportRecord={sticky.exportRecord}
      onRetry={sticky.retry}
      onDismissError={sticky.dismissError}
    />
  );
}
