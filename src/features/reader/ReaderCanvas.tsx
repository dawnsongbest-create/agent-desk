import ReactMarkdown, { type Components } from "react-markdown";
import type {
  ReaderFontSize,
  ReaderLineSpacing,
  ReaderSkin,
  WindowPreset,
} from "../../domain/preferences";
import { readerFixture } from "./readerFixture";

type ReaderCanvasProps = {
  skin: ReaderSkin;
  fontSize: ReaderFontSize;
  lineSpacing: ReaderLineSpacing;
  windowPreset: WindowPreset;
  markdown?: string;
};

const markdownComponents: Components = {
  a: ({ children, href }) => (
    <a href={href} target="_blank" rel="noreferrer">
      {children}
    </a>
  ),
};

export function ReaderCanvas({
  skin,
  fontSize,
  lineSpacing,
  windowPreset,
  markdown = readerFixture,
}: ReaderCanvasProps) {
  return (
    <section
      className="reader-canvas"
      data-reader-layer="1"
      data-reader-skin={skin}
      data-font-size={fontSize}
      data-line-spacing={lineSpacing}
      data-window-preset={windowPreset}
      aria-label="Reader Canvas"
      tabIndex={0}
    >
      <div className="reader-top-safe-area" data-testid="reader-top-safe-area" aria-hidden="true" />
      <article className="reader-content" aria-label="Reader 示例文章">
        <p className="reader-context">M2-A · 本地阅读示例 · 约 8 分钟</p>
        <div className="reader-article">
          <ReactMarkdown skipHtml components={markdownComponents}>
            {markdown}
          </ReactMarkdown>
        </div>
        <p className="reader-end-mark" aria-hidden="true">
          ✦
        </p>
      </article>
    </section>
  );
}
