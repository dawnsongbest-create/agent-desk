import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import appCss from "../../App.css?raw";
import type { WindowPreset } from "../../domain/preferences";
import { ReaderCanvas } from "./ReaderCanvas";

const typographyFixture = `# 主标题

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

function renderReader(windowPreset: WindowPreset = "iphone5", markdown = typographyFixture) {
  return render(
    <ReaderCanvas
      skin="grid"
      fontSize="standard"
      lineSpacing="standard"
      windowPreset={windowPreset}
      markdown={markdown}
    />,
  );
}

describe("ReaderCanvas M2-A foundation", () => {
  it("starts content after an explicit top safe area", () => {
    renderReader();
    const reader = screen.getByRole("region", { name: "Reader Canvas" });
    const safeArea = screen.getByTestId("reader-top-safe-area");
    const content = within(reader).getByRole("article", { name: "Reader 示例文章" });
    expect(reader).toHaveAttribute("data-reader-layer", "1");
    expect(safeArea.compareDocumentPosition(content) & Node.DOCUMENT_POSITION_FOLLOWING).not.toBe(
      0,
    );
  });

  it("renders the required Markdown typography without raw HTML injection", () => {
    const { container } = renderReader();
    expect(screen.getByRole("heading", { level: 1, name: "主标题" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: "二级标题" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3, name: "三级标题" })).toBeInTheDocument();
    expect(container.querySelector("strong")).toHaveTextContent("强调");
    expect(container.querySelector("em")).toHaveTextContent("斜体");
    expect(container.querySelector("ul")).toBeInTheDocument();
    expect(container.querySelector("ol")).toBeInTheDocument();
    expect(container.querySelector("blockquote")).toHaveTextContent("一段纸上的引用");
    expect(container.querySelector("p code")).toHaveTextContent("inline code");
    expect(container.querySelector("pre code")).toHaveTextContent("const safe = true;");
    expect(container.querySelector("hr")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "安全链接" })).toHaveAttribute(
      "href",
      "https://commonmark.org/",
    );
  });

  it("drops unsafe raw HTML instead of executing or rendering it directly", () => {
    const { container } = renderReader(
      "iphone5",
      '# 安全正文\n\n<img src=x onerror="window.readerUnsafe=true">\n\n<script>window.readerUnsafe=true</script>',
    );
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

  it("keeps 320px layouts bounded with responsive padding and horizontal containment", () => {
    expect(appCss).toContain("overflow-x: hidden");
    expect(appCss).toContain("padding: 0 clamp(18px, 7vw, 38px)");
    expect(appCss).toContain("max-width: 100%");
    expect(appCss).toContain("overflow-x: auto");
  });
});
