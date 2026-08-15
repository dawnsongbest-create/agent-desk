import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import appCss from "../../App.css?raw";
import type { WindowPreset } from "../../domain/preferences";
import type { ReaderDocument } from "../../domain/reader";
import { ReaderCanvas } from "./ReaderCanvas";

const typographyFixture = `# 文中主标题

正文包含 **强调**、*斜体* 和一段 \`inline code\`。

## 二级标题

### 三级标题

- 无序一
- 无序二

1. 有序一
2. 有序二

> 一段纸上的引用。

~~~ts
const safe = true;
~~~

---

[安全链接](https://commonmark.org/)
`;

const readerDocument: ReaderDocument = {
  id: "reader-test",
  documentType: "article",
  title: "真实 Reader 文档",
  subtitle: "由 document model 提供的副标题",
  contentMarkdown: typographyFixture,
  sourceType: "builtin",
  sourceLabel: "本地测试",
  createdAt: "2026-08-15T00:00:00Z",
  updatedAt: "2026-08-15T00:00:00Z",
};

function renderReader(
  windowPreset: WindowPreset = "iphone5",
  document: ReaderDocument = readerDocument,
  onCopy = vi.fn(async () => undefined),
  onCaptureSelection = vi.fn(async () => true),
) {
  return {
    ...render(
      <ReaderCanvas
        skin="grid"
        fontSize="standard"
        lineSpacing="standard"
        windowPreset={windowPreset}
        document={document}
        state="ready"
        onRetry={vi.fn()}
        onCopy={onCopy}
        onCaptureSelection={onCaptureSelection}
      />,
    ),
    onCopy,
    onCaptureSelection,
  };
}

function mockSelection(
  anchorNode: Node,
  text: string,
  rect: Partial<DOMRect> = { left: 120, top: 180, right: 220, bottom: 200, width: 100, height: 20 },
) {
  const bounds = {
    x: rect.left ?? 0,
    y: rect.top ?? 0,
    left: rect.left ?? 0,
    top: rect.top ?? 0,
    right: rect.right ?? 0,
    bottom: rect.bottom ?? 0,
    width: rect.width ?? 0,
    height: rect.height ?? 0,
    toJSON: () => ({}),
  } as DOMRect;
  return vi.spyOn(window, "getSelection").mockReturnValue({
    rangeCount: 1,
    isCollapsed: false,
    anchorNode,
    focusNode: anchorNode,
    toString: () => text,
    getRangeAt: () => ({ getBoundingClientRect: () => bounds }),
  } as unknown as Selection);
}

afterEach(() => vi.restoreAllMocks());

describe("ReaderCanvas document and selection", () => {
  it("renders a document identity and starts it after the responsive safe area", () => {
    renderReader();
    const reader = screen.getByRole("region", { name: "Reader Canvas" });
    const safeArea = screen.getByTestId("reader-top-safe-area");
    const content = within(reader).getByRole("article", { name: "真实 Reader 文档" });
    expect(reader).toHaveAttribute("data-current-document-id", "reader-test");
    expect(reader).toHaveAttribute("data-reader-layer", "1");
    expect(safeArea.compareDocumentPosition(content) & Node.DOCUMENT_POSITION_FOLLOWING).not.toBe(
      0,
    );
    expect(screen.getByRole("heading", { level: 1, name: "真实 Reader 文档" })).toBeVisible();
    expect(screen.getByText("文章 · 本地测试")).toBeVisible();
  });

  it("renders required Markdown typography without raw HTML injection", () => {
    const { container } = renderReader();
    expect(screen.getByRole("heading", { level: 1, name: "文中主标题" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: "二级标题" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3, name: "三级标题" })).toBeInTheDocument();
    expect(container.querySelector("strong")).toHaveTextContent("强调");
    expect(container.querySelector("em")).toHaveTextContent("斜体");
    expect(container.querySelector("blockquote")).toHaveTextContent("一段纸上的引用");
    expect(container.querySelector("p code")).toHaveTextContent("inline code");
    expect(container.querySelector("pre code")).toHaveTextContent("const safe = true;");
    expect(container.querySelector("hr")).toBeInTheDocument();
  });

  it("drops unsafe raw HTML", () => {
    const unsafe = {
      ...readerDocument,
      contentMarkdown:
        '<img src=x onerror="window.readerUnsafe=true">\n\n<script>window.readerUnsafe=true</script>',
    };
    const { container } = renderReader("iphone5", unsafe);
    expect(container.querySelector("img")).not.toBeInTheDocument();
    expect(container.querySelector("script")).not.toBeInTheDocument();
    expect(container.innerHTML).not.toContain("onerror");
  });

  it.each(["sticky", "iphone5", "pocket", "book"] as const)(
    "renders the %s preset through the same responsive canvas",
    (preset) => {
      renderReader(preset);
      expect(screen.getByRole("region", { name: "Reader Canvas" })).toHaveAttribute(
        "data-window-preset",
        preset,
      );
    },
  );

  it.each(["", " \n\t "])("does not show actions for empty selection %j", (selectedText) => {
    renderReader();
    const article = screen.getByRole("article", { name: "真实 Reader 文档" });
    mockSelection(article.firstChild!, selectedText);
    fireEvent.mouseUp(article);
    expect(screen.queryByRole("toolbar", { name: "所选文字操作" })).not.toBeInTheDocument();
  });

  it("shows only Copy and Save actions for a Reader text selection", () => {
    renderReader();
    const text = screen.getByText(/正文包含/).firstChild!;
    mockSelection(text, "强调与原文");
    fireEvent.mouseUp(text.parentElement!);
    const toolbar = screen.getByRole("toolbar", { name: "所选文字操作" });
    expect(within(toolbar).getByRole("button", { name: "复制" })).toBeVisible();
    expect(within(toolbar).getByRole("button", { name: "保存到记录" })).toBeVisible();
    expect(within(toolbar).getAllByRole("button")).toHaveLength(2);
  });

  it("ignores a selection whose endpoints belong to Sticky UI", () => {
    const { container } = renderReader();
    const sticky = globalThis.document.createElement("span");
    sticky.textContent = "Sticky text";
    container.append(sticky);
    const article = screen.getByRole("article", { name: "真实 Reader 文档" });
    mockSelection(sticky.firstChild!, "Sticky text");
    fireEvent.mouseUp(article);
    expect(screen.queryByRole("toolbar", { name: "所选文字操作" })).not.toBeInTheDocument();
  });

  it.each([
    ["top", { left: 4, right: 48, top: 2, bottom: 18, width: 44, height: 16 }],
    ["right", { left: 1000, right: 1020, top: 120, bottom: 140, width: 20, height: 20 }],
  ] as const)("keeps the popover inside the viewport near the %s edge", (_edge, rect) => {
    renderReader();
    const article = screen.getByRole("article", { name: "真实 Reader 文档" });
    mockSelection(article.firstChild!, "边缘选择", rect);
    fireEvent.mouseUp(article);
    const toolbar = screen.getByRole("toolbar", { name: "所选文字操作" });
    expect(Number.parseFloat(toolbar.style.left)).toBeGreaterThanOrEqual(8);
    expect(Number.parseFloat(toolbar.style.top)).toBeGreaterThanOrEqual(8);
  });

  it("closes selection actions on Reader scroll and when selection is lost", () => {
    renderReader();
    const article = screen.getByRole("article", { name: "真实 Reader 文档" });
    mockSelection(article.firstChild!, "滚动前选择");
    fireEvent.mouseUp(article);
    fireEvent.scroll(screen.getByRole("region", { name: "Reader Canvas" }));
    expect(screen.queryByRole("toolbar", { name: "所选文字操作" })).not.toBeInTheDocument();

    const selection = mockSelection(article.firstChild!, "再次选择");
    fireEvent.mouseUp(article);
    selection.mockReturnValue(null);
    fireEvent(globalThis.document, new Event("selectionchange"));
    expect(screen.queryByRole("toolbar", { name: "所选文字操作" })).not.toBeInTheDocument();
  });

  it.each(["中文原文", "English exact text", "第一行\n\n第二行", "const exact = true;"])(
    "copies exact selected text: %s",
    async (selectedText) => {
      const user = userEvent.setup();
      const { onCopy } = renderReader();
      const article = screen.getByRole("article", { name: "真实 Reader 文档" });
      mockSelection(article.firstChild!, selectedText);
      fireEvent.mouseUp(article);
      await user.click(screen.getByRole("button", { name: "复制" }));
      expect(onCopy).toHaveBeenCalledWith(selectedText);
      expect(screen.getByRole("status")).toHaveTextContent("已复制");
    },
  );

  it("saves exact multiline selection against the current document without interrupting Reader", async () => {
    const user = userEvent.setup();
    const selectedText = "第一段原文\n\nSecond paragraph\nconst value = 1;";
    const { onCaptureSelection } = renderReader();
    const article = screen.getByRole("article", { name: "真实 Reader 文档" });
    mockSelection(article.firstChild!, selectedText);
    fireEvent.mouseUp(article);
    await user.click(screen.getByRole("button", { name: "保存到记录" }));
    expect(onCaptureSelection).toHaveBeenCalledWith("reader-test", selectedText);
    expect(screen.getByRole("status")).toHaveTextContent("✓ 已保存到记录");
    expect(article).toBeInTheDocument();
  });

  it("does not let an earlier copy confirmation close a newer selection", async () => {
    vi.useFakeTimers();
    const { onCopy } = renderReader();
    const article = screen.getByRole("article", { name: "真实 Reader 文档" });
    const firstSelection = mockSelection(article.firstChild!, "第一次选择");
    fireEvent.mouseUp(article);
    fireEvent.click(screen.getByRole("button", { name: "复制" }));
    await vi.waitFor(() => expect(onCopy).toHaveBeenCalledWith("第一次选择"));

    firstSelection.mockReturnValue({
      rangeCount: 1,
      isCollapsed: false,
      anchorNode: article.firstChild,
      focusNode: article.firstChild,
      toString: () => "新的选择",
      getRangeAt: () => ({
        getBoundingClientRect: () =>
          ({ left: 120, top: 180, right: 220, bottom: 200, width: 100, height: 20 }) as DOMRect,
      }),
    } as unknown as Selection);
    fireEvent.mouseUp(article);
    vi.advanceTimersByTime(800);
    expect(screen.getByRole("toolbar", { name: "所选文字操作" })).toBeInTheDocument();
    vi.useRealTimers();
  });

  it("keeps the selection available for retry after a save failure", async () => {
    const user = userEvent.setup();
    const onCapture = vi.fn(async () => false);
    renderReader("iphone5", readerDocument, vi.fn(), onCapture);
    const article = screen.getByRole("article", { name: "真实 Reader 文档" });
    mockSelection(article.firstChild!, "不要丢失");
    fireEvent.mouseUp(article);
    await user.click(screen.getByRole("button", { name: "保存到记录" }));
    expect(screen.getByRole("status")).toHaveTextContent("操作失败，请重试");
    expect(screen.getByRole("button", { name: "保存到记录" })).toBeEnabled();
  });

  it("keeps 320px layouts bounded with responsive padding and horizontal containment", () => {
    expect(appCss).toContain("overflow-x: hidden");
    expect(appCss).toContain("padding: 0 clamp(18px, 7vw, 38px)");
    expect(appCss).toContain("max-width: 100%");
    expect(appCss).toContain("overflow-x: auto");
  });
});
