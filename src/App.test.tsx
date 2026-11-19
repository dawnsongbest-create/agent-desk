import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultPreferences } from "./domain/preferences";

const mocks = vi.hoisted(() => ({
  loadPreferences: vi.fn(),
  savePreferences: vi.fn(),
  applyWindowPreset: vi.fn(),
  listCards: vi.fn(),
  getProfile: vi.fn(),
  openReader: vi.fn(),
  captureSelection: vi.fn(),
  copyText: vi.fn(),
  getAgentBridgeStatus: vi.fn(),
  enterDesktopSticky: vi.fn(),
  openReaderWorkspace: vi.fn(),
  setDesktopStickyMode: vi.fn(),
  setReaderStickyMode: vi.fn(),
  leaveReader: vi.fn(),
  listReadingPlans: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(vi.fn()) }));

vi.mock("./infrastructure/tauri/preferences", () => ({
  tauriPreferences: {
    load: mocks.loadPreferences,
    save: mocks.savePreferences,
    applyWindowPreset: mocks.applyWindowPreset,
  },
}));

vi.mock("./infrastructure/tauri/reading", () => ({
  tauriReadingPlans: { listPlans: mocks.listReadingPlans, createSession: vi.fn() },
}));

vi.mock("./infrastructure/tauri/sticky", () => ({
  tauriStickyCards: {
    list: mocks.listCards,
    getProfile: mocks.getProfile,
    create: vi.fn(),
    updateText: vi.fn(),
    setTaskCompleted: vi.fn(),
    setTaskDueDate: vi.fn(),
    delete: vi.fn(),
    reorder: vi.fn(),
    updateQuote: vi.fn(),
    exportRecord: vi.fn(),
  },
}));

vi.mock("./infrastructure/tauri/reader", () => ({
  tauriReaderDocuments: {
    openCurrent: mocks.openReader,
    get: vi.fn(),
    list: vi.fn(),
    create: vi.fn(),
    captureSelection: mocks.captureSelection,
    copyText: mocks.copyText,
  },
}));

vi.mock("./infrastructure/tauri/agentConnection", () => ({
  tauriAgentConnection: {
    getStatus: mocks.getAgentBridgeStatus,
    start: vi.fn(),
    stop: vi.fn(),
    generateToken: vi.fn(),
    copyToken: vi.fn(),
  },
}));

vi.mock("./infrastructure/tauri/desktopSticky", () => ({
  tauriDesktopSticky: {
    enter: mocks.enterDesktopSticky,
    open: vi.fn(),
    openReader: mocks.openReaderWorkspace,
    setDesktopMode: mocks.setDesktopStickyMode,
    setReaderMode: mocks.setReaderStickyMode,
    leaveReader: mocks.leaveReader,
    startDragging: vi.fn(),
    showContextMenu: vi.fn(),
  },
}));

import App from "./App";

describe("App Reader preferences", () => {
  it("opens Reader for an obsolete plans view without loading plan management", async () => {
    mocks.loadPreferences.mockResolvedValue({
      ...defaultPreferences,
      readerSurfaceMode: "plans",
    });
    render(<App />);
    expect(await screen.findByRole("article", { name: "测试文档" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "打开阅读计划" })).not.toBeInTheDocument();
    expect(mocks.listReadingPlans).not.toHaveBeenCalled();
  });
  it("persists visibility using a base snapshot and restores a hidden Reader", async () => {
    const user = userEvent.setup();
    const view = render(<App />);
    await screen.findByRole("article");
    await user.click(screen.getByRole("button", { name: "隐藏便利贴", pressed: true }));
    await waitFor(() =>
      expect(mocks.savePreferences).toHaveBeenLastCalledWith(
        expect.objectContaining({ readerStickyVisible: false }),
        expect.objectContaining({ readerStickyVisible: true }),
      ),
    );
    expect(document.querySelector(".sticky-overlay-layer")).toHaveAttribute("hidden");
    expect(document.querySelector(".mini-tab")).toBeNull();
    view.unmount();
    mocks.loadPreferences.mockResolvedValue({ ...defaultPreferences, readerStickyVisible: false });
    render(<App />);
    await screen.findByRole("article");
    expect(screen.getByRole("button", { name: "显示便利贴" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.loadPreferences.mockResolvedValue(defaultPreferences);
    mocks.savePreferences.mockImplementation(async (preferences) => preferences);
    mocks.listCards.mockResolvedValue([]);
    mocks.getProfile.mockResolvedValue({ quoteText: "", updatedAt: "" });
    mocks.openReader.mockResolvedValue({
      id: "reader-test",
      documentType: "article",
      title: "测试文档",
      subtitle: null,
      contentMarkdown: "正文",
      sourceType: "builtin",
      sourceLabel: "测试",
      createdAt: "2026-08-12T00:00:00Z",
      updatedAt: "2026-08-12T00:00:00Z",
    });
    mocks.getAgentBridgeStatus.mockResolvedValue({
      version: "v1",
      enabled: false,
      running: false,
      bindAddress: "127.0.0.1",
      port: 47321,
      endpoint: "http://127.0.0.1:47321/api/v1",
      connection: null,
      lastError: null,
    });
    mocks.enterDesktopSticky.mockResolvedValue(defaultPreferences);
    mocks.setDesktopStickyMode.mockImplementation(async (stickyMode) => ({
      ...defaultPreferences,
      surfaceMode: "desktop",
      stickyMode,
    }));
    mocks.setReaderStickyMode.mockImplementation(async (stickyMode) => ({
      ...defaultPreferences,
      surfaceMode: "reader",
      stickyMode,
    }));
    mocks.leaveReader.mockImplementation(async (stickyMode) => ({
      ...defaultPreferences,
      surfaceMode: "desktop",
      stickyMode,
    }));
    mocks.openReaderWorkspace.mockResolvedValue({
      ...defaultPreferences,
      surfaceMode: "reader",
    });
  });

  it("persists font size and line spacing through the Preference Port", async () => {
    const user = userEvent.setup();
    render(<App />);

    const reader = await screen.findByRole("region", { name: "Reader Canvas" });
    expect(reader).toHaveAttribute("data-font-size", "standard");
    expect(reader).toHaveAttribute("data-line-spacing", "standard");

    await user.click(screen.getByRole("button", { name: "外观与窗口设置" }));
    await user.click(within(screen.getByRole("group", { name: "Reader 字号" })).getByText("大"));

    await waitFor(() => {
      expect(mocks.savePreferences.mock.lastCall?.[0]).toEqual({
        ...defaultPreferences,
        currentReaderDocumentId: "reader-test",
        readerFontSize: "large",
      });
      expect(reader).toHaveAttribute("data-font-size", "large");
    });

    await user.click(within(screen.getByRole("group", { name: "Reader 行距" })).getByText("宽松"));

    await waitFor(() => {
      expect(mocks.savePreferences.mock.lastCall?.[0]).toEqual({
        ...defaultPreferences,
        currentReaderDocumentId: "reader-test",
        readerFontSize: "large",
        readerLineSpacing: "relaxed",
      });
      expect(reader).toHaveAttribute("data-line-spacing", "relaxed");
    });
  });

  it("restores stored Reader preferences when the application starts", async () => {
    mocks.loadPreferences.mockResolvedValue({
      ...defaultPreferences,
      readerFontSize: "small",
      readerLineSpacing: "compact",
    });

    render(<App />);

    const reader = await screen.findByRole("region", { name: "Reader Canvas" });
    await waitFor(() => {
      expect(reader).toHaveAttribute("data-font-size", "small");
      expect(reader).toHaveAttribute("data-line-spacing", "compact");
    });
  });

  it("persists blank Reader mode and restores it across application restart", async () => {
    const user = userEvent.setup();
    const first = render(<App />);
    await screen.findByRole("article", { name: "测试文档" });
    await waitFor(() =>
      expect(mocks.savePreferences.mock.lastCall?.[0]).toEqual({
        ...defaultPreferences,
        currentReaderDocumentId: "reader-test",
      }),
    );
    vi.clearAllMocks();

    await user.click(screen.getByRole("button", { name: "隐藏正文" }));
    expect(screen.queryByRole("article")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "显示正文" })).toBeVisible();
    await waitFor(() =>
      expect(mocks.savePreferences.mock.lastCall?.[0]).toEqual({
        ...defaultPreferences,
        currentReaderDocumentId: "reader-test",
        readerContentVisible: false,
      }),
    );

    first.unmount();
    vi.clearAllMocks();
    mocks.loadPreferences.mockResolvedValue({
      ...defaultPreferences,
      stickyMode: "mini",
      readerContentVisible: false,
      currentReaderDocumentId: "reader-test",
    });
    mocks.savePreferences.mockImplementation(async (preferences) => preferences);

    render(<App />);
    await screen.findByRole("region", { name: "Reader Canvas" });
    await waitFor(() => expect(mocks.openReader).toHaveBeenCalledWith("reader-test"));
    expect(screen.queryByRole("article")).not.toBeInTheDocument();
    expect(document.querySelector(".mini-tab")).toBeNull();

    await user.click(screen.getByRole("button", { name: "显示正文" }));
    expect(await screen.findByRole("article", { name: "测试文档" })).toBeVisible();
    await waitFor(() =>
      expect(mocks.savePreferences.mock.lastCall?.[0]).toEqual({
        ...defaultPreferences,
        stickyMode: "mini",
        readerContentVisible: true,
        currentReaderDocumentId: "reader-test",
      }),
    );
  });

  it("persists the bootstrapped current document id and reopens by that id", async () => {
    const first = render(<App />);
    await waitFor(() =>
      expect(mocks.savePreferences.mock.lastCall?.[0]).toEqual({
        ...defaultPreferences,
        currentReaderDocumentId: "reader-test",
      }),
    );
    first.unmount();
    vi.clearAllMocks();
    mocks.loadPreferences.mockResolvedValue({
      ...defaultPreferences,
      currentReaderDocumentId: "reader-test",
    });
    mocks.savePreferences.mockImplementation(async (preferences) => preferences);
    mocks.listCards.mockResolvedValue([]);
    mocks.getProfile.mockResolvedValue({ quoteText: "", updatedAt: "" });
    mocks.openReader.mockResolvedValue({
      id: "reader-test",
      documentType: "article",
      title: "测试文档",
      subtitle: null,
      contentMarkdown: "正文",
      sourceType: "builtin",
      sourceLabel: "测试",
      createdAt: "2026-08-12T00:00:00Z",
      updatedAt: "2026-08-12T00:00:00Z",
    });

    render(<App />);
    await waitFor(() => expect(mocks.openReader).toHaveBeenCalledWith("reader-test"));
    expect(mocks.savePreferences).not.toHaveBeenCalled();
  });

  it("leaves hidden Reader Sticky as Desktop Compact without a Reader Mini", async () => {
    const user = userEvent.setup();
    mocks.loadPreferences.mockResolvedValue({
      ...defaultPreferences,
      stickyMode: "compact",
      readerStickyVisible: false,
      currentReaderDocumentId: "reader-test",
    });

    render(<App />);
    await screen.findByRole("region", { name: "Reader Canvas" });
    await user.click(await screen.findByRole("button", { name: "只留便利贴" }));

    await waitFor(() => expect(mocks.leaveReader).toHaveBeenCalledWith("compact"));
  });

  it("does not render the removed Gateway page", async () => {
    const user = userEvent.setup();
    mocks.loadPreferences.mockResolvedValue({
      ...defaultPreferences,
      surfaceMode: "desktop",
      currentReaderDocumentId: "reader-test",
    });
    mocks.openReaderWorkspace.mockResolvedValue({
      ...defaultPreferences,
      surfaceMode: "reader",
      currentReaderDocumentId: "reader-test",
    });

    render(<App surface="desktop" />);

    expect(await screen.findByText("TODAY")).toBeVisible();
    expect(screen.queryByText("你的阅读工作空间")).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Reader Canvas" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "打开阅读器" }));

    await waitFor(() => expect(mocks.openReaderWorkspace).toHaveBeenCalledOnce());
  });

  it("persists the three-way startup display preference", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("region", { name: "Reader Canvas" });

    await user.click(screen.getByRole("button", { name: "外观与窗口设置" }));
    await user.click(
      within(screen.getByRole("group", { name: "启动显示模式" })).getByText("便利贴贴在阅读器上"),
    );

    await waitFor(() =>
      expect(mocks.savePreferences).toHaveBeenLastCalledWith(
        expect.objectContaining({ startupDisplayMode: "sticky_over_reader" }),
        expect.any(Object),
      ),
    );
  });
});
