import { listen } from "@tauri-apps/api/event";
import {
  act,
  createEvent,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
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
  startDragging: vi.fn(),
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
    startDragging: mocks.startDragging,
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
  it("folds Reader Compact to a pinned and persistent Mini; restores Compact", async () => {
    const user = userEvent.setup();
    const view = render(<App />);
    const article = await screen.findByRole("article");
    await user.click(screen.getByRole("button", { name: "缩成 Mini Tab" }));
    const mini = await screen.findByRole("button", { name: "恢复 Compact Sticky" });
    expect(mini.closest(".sticky-surface")).toHaveAttribute("data-pinned-to-shelf", "true");
    expect(article).toBeVisible();
    expect(mocks.setReaderStickyMode).toHaveBeenLastCalledWith("mini");
    expect(mocks.setDesktopStickyMode).not.toHaveBeenCalled();
    expect(mocks.leaveReader).not.toHaveBeenCalled();
    view.unmount();
    mocks.loadPreferences.mockResolvedValue({
      ...defaultPreferences,
      stickyMode: "mini",
      currentReaderDocumentId: "reader-test",
    });
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "恢复 Compact Sticky" }));
    expect(await screen.findByRole("button", { name: "展开或拖动便利贴" })).toBeVisible();
    expect(mocks.setReaderStickyMode).toHaveBeenLastCalledWith("compact");
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
    mocks.startDragging.mockResolvedValue(undefined);
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
    expect(screen.getByRole("button", { name: "恢复 Compact Sticky" })).toBeVisible();

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

  it.each(["mini", "compact", "expanded"] as const)(
    "leaves Reader %s in the same shape",
    async (stickyMode) => {
      const user = userEvent.setup();
      mocks.loadPreferences.mockResolvedValue({
        ...defaultPreferences,
        stickyMode,
        currentReaderDocumentId: "reader-test",
        readerScrollTop: 240,
      });

      render(<App />);
      await screen.findByRole("article");
      await user.click(screen.getByRole("button", { name: "只留便利贴" }));
      await waitFor(() => expect(mocks.leaveReader).toHaveBeenCalledWith(stickyMode));
    },
  );

  it.each(["click", "Enter", "Space"] as const)(
    "opens Reader from Compact header with %s only",
    async (interaction) => {
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
      const reader = screen.getByRole("button", { name: "打开阅读器" });
      expect(reader.closest(".compact-header")).not.toBeNull();
      expect(reader).toHaveTextContent("阅读");
      expect(reader).toHaveAttribute("title", "打开阅读器");
      if (interaction === "click") await user.click(reader);
      else {
        reader.focus();
        await user.keyboard(interaction === "Enter" ? "{Enter}" : " ");
      }

      await waitFor(() => expect(mocks.openReaderWorkspace).toHaveBeenCalledOnce());
      expect(mocks.setDesktopStickyMode).not.toHaveBeenCalled();
      expect(mocks.startDragging).not.toHaveBeenCalled();
      const compact = screen.getByRole("button", { name: "展开或拖动便利贴" });
      for (const [eventName, x] of [
        ["pointerDown", 90],
        ["pointerMove", 110],
        ["pointerUp", 110],
      ] as const) {
        const event = createEvent[eventName](compact);
        Object.defineProperties(event, {
          pointerId: { value: 1 },
          clientX: { value: x },
          clientY: { value: 100 },
        });
        fireEvent(compact, event);
      }
      expect(mocks.startDragging).toHaveBeenCalledOnce();
      expect(mocks.openReaderWorkspace).toHaveBeenCalledOnce();
      expect(mocks.setDesktopStickyMode).not.toHaveBeenCalled();
    },
  );

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

  it("cycles Desktop Compact, Mini and Expanded without entering Reader", async () => {
    const user = userEvent.setup();
    mocks.loadPreferences.mockResolvedValue({ ...defaultPreferences, surfaceMode: "desktop" });
    render(<App surface="desktop" />);
    const fold = await screen.findByRole("button", { name: "缩成 Mini Tab" });
    await user.click(fold);
    const restore = await screen.findByRole("button", { name: "恢复 Compact Sticky" });
    expect(restore.closest(".sticky-surface")).toHaveAttribute("data-pinned-to-shelf", "false");
    restore.focus();
    await user.keyboard("{Enter}");
    await user.click(await screen.findByRole("button", { name: "展开或拖动便利贴" }));
    expect(await screen.findByRole("region", { name: "展开的便利贴" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "收起便利贴" }));
    await user.click(await screen.findByRole("button", { name: "展开或拖动便利贴" }));
    await user.click(await screen.findByRole("button", { name: "缩成 Mini Tab" }));
    expect(await screen.findByRole("button", { name: "恢复 Compact Sticky" })).toBeVisible();
    expect(mocks.setDesktopStickyMode.mock.calls.flat()).toEqual([
      "mini",
      "compact",
      "expanded",
      "compact",
      "expanded",
      "mini",
    ]);
    expect(mocks.openReaderWorkspace).not.toHaveBeenCalled();
    expect(mocks.startDragging).not.toHaveBeenCalled();
  });

  it.each(["desktop", "main"] as const)(
    "refreshes sticky content after returning to %s",
    async (surface) => {
      mocks.loadPreferences.mockResolvedValue({
        ...defaultPreferences,
        surfaceMode: surface === "desktop" ? "desktop" : "reader",
      });
      render(<App surface={surface} />);
      await screen.findByText("留一句愿意常常看到的话。");
      mocks.getProfile.mockResolvedValue({ quoteText: "Saved in the other window", updatedAt: "" });
      const event = "agent-desk://sticky-content-refresh";
      const callback = vi.mocked(listen).mock.calls.find(([name]) => name === event)?.[1];
      expect(callback).toBeDefined();
      await act(async () => callback?.({ event, id: 1, payload: null }));
      expect(await screen.findByText("Saved in the other window")).toBeVisible();
      expect(mocks.listCards).toHaveBeenCalledTimes(2);
    },
  );
});
