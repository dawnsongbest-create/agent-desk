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
}));

vi.mock("./infrastructure/tauri/preferences", () => ({
  tauriPreferences: {
    load: mocks.loadPreferences,
    save: mocks.savePreferences,
    applyWindowPreset: mocks.applyWindowPreset,
  },
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

import App from "./App";

describe("App Reader preferences", () => {
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
      expect(mocks.savePreferences).toHaveBeenCalledWith({
        ...defaultPreferences,
        currentReaderDocumentId: "reader-test",
        readerFontSize: "large",
      });
      expect(reader).toHaveAttribute("data-font-size", "large");
    });

    await user.click(within(screen.getByRole("group", { name: "Reader 行距" })).getByText("宽松"));

    await waitFor(() => {
      expect(mocks.savePreferences).toHaveBeenLastCalledWith({
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
      expect(mocks.savePreferences).toHaveBeenCalledWith({
        ...defaultPreferences,
        currentReaderDocumentId: "reader-test",
      }),
    );
    vi.clearAllMocks();

    await user.click(screen.getByRole("button", { name: "隐藏正文" }));
    expect(screen.queryByRole("article")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "显示正文" })).toBeVisible();
    await waitFor(() =>
      expect(mocks.savePreferences).toHaveBeenCalledWith({
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
      expect(mocks.savePreferences).toHaveBeenCalledWith({
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
      expect(mocks.savePreferences).toHaveBeenCalledWith({
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
});
