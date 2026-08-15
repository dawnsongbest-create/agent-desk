# Agent Desk — M2-A Reader Canvas Foundation Gate Report

## A. Implementation Summary

M2-A upgrades the prior Background Board into a real Reader Canvas while preserving the closed M1
Sticky product. The implementation adds a long local Markdown presentation fixture, publication-style
typography, a responsive top safe area, a quiet procedural paper texture, Reader-owned scrolling, and
Reader font/line-spacing preferences in the existing Appearance panel.

The delivered product remains deliberately narrow:

- no Reader domain model, repository, SQLite table, progress, Inbox, Agent delivery, search, TOC,
  annotation, highlight, summarization, notification, scheduling, or cloud behavior was added;
- the only Reader skin is `grid`; `ReaderSkin` is extensible, but there is no Kraft UI, disabled button,
  or fake preview;
- fixture content is a React-side module and never enters SQLite;
- M1 Record, Todo, Quote, Mini/Compact/Expanded, drag/snap, page turn, audio, export, and three-line
  Quote behavior remain in place.

Final result: **PASS**.

## B. Reader Canvas Structure

The production structure is:

```text
Window / paper background       Layer 0
└── Reader Canvas               Layer 1 (z-index 1, primary scroll owner)
    ├── responsive top safe area
    └── centered content column
└── Sticky overlay              Layer 2 (z-index 20)
    ├── Mini
    ├── Compact
    └── Expanded
└── existing board controls     z-index 30
```

Reader and Sticky are absolute sibling layers. Sticky never participates in Reader document flow;
there is no text wrapping, collision avoidance, or position-driven Reader reflow. Native computed
attributes were `data-reader-layer=1` and `data-reader-layer=2`, with z-indices `1` and `20`.

## C. Grid Paper Changes

The accepted 24px grid direction is unchanged. Only line alpha changed:

| Theme | Before | M2-A | Relative alpha change |
| --- | ---: | ---: | ---: |
| Light | 5.0% | 5.6% | +12.0% |
| Dark / system-dark | 4.0% | 4.5% | +12.5% |

The final Windows smoke ran under system-dark and read the production variable as
`rgb(235 225 200 / 4.5%)`. Line width, spacing, hue direction, and grid geometry were not enlarged.

## D. Top Safe Area

Option B is implemented as an explicit element before Reader content. It is responsive:

```css
height: clamp(136px, 31vh, 190px);
```

The 320×420 short-window override is 140px. Native results:

| Preset | Viewport | Safe area | Title top |
| --- | ---: | ---: | ---: |
| Sticky | 320×420 | 140.0px | 162.4px |
| iPhone 5 | 320×568 | 176.1px | 198.5px |
| Pocket | 360×640 | 190.0px | 212.4px |
| Book | 420×595 reported by WebView2 | 184.5px | 206.9px |

For the required iPhone 5 default transition, Mini bottom was 182.0px and title top was 198.5px:
the first title remained clear by 16.4px. A user can still deliberately drag Mini into content.

## E. Typography System

The local fixture and automated renderer checks cover:

- H1, H2, H3;
- paragraphs with Chinese, English, numerals, strong, and emphasis;
- unordered and ordered lists, including bounded nested-list rules;
- paper-like blockquote with a light left rule and tonal wash;
- inline code and horizontally scrollable code blocks;
- a low-contrast horizontal rule;
- low-saturation links with visible hover and keyboard focus.

Body text uses theme-aware deep gray rather than pure black. The content column is capped at 640px,
while its responsive padding protects usable width at 320px. Code blocks have no syntax engine,
line numbers, copy button, heavy dark surface, or IDE framing.

## F. Markdown Renderer

`react-markdown` 10.1.0 is used as a presentation-only renderer. The installed package is MIT
licensed. It is a mature React Markdown renderer and does not add an editor or remote runtime.

Safety constraints:

- `skipHtml` is enabled;
- unsafe raw HTML is neither mounted nor executed in tests;
- no remote scripts or content fetches are performed;
- links are rendered with `rel="noreferrer"`;
- the fixture is a local TypeScript constant, not Agent delivery.

## G. Paper Texture

Paper texture is a procedural CSS radial fiber at 6px spacing. Its color alpha is 2% in light mode
and 1.5% in dark mode. There is no image asset, network request, stain, fold, aging effect, heavy noise,
or heavy shadow. Visual QA confirmed text remains substantially more prominent than texture.

## H. Reader Scroll Behavior

`.reader-canvas` is the primary vertical scroll owner with `overflow-y: auto`, keyboard focusability,
thin low-contrast scrollbars, and stronger scrollbar feedback on hover/focus. Horizontal overflow is
contained; long code scrolls inside its own block.

The existing board header now has a paper-color fade so mid-article text does not strongly collide
with the persistent date/controls. Its non-control area uses `pointer-events: none`, allowing wheel
input to reach Reader; the Appearance button remains interactive.

Native long-scroll reached `scrollTop=2450.4`. All four presets accepted Reader scrolling and had no
document or Reader horizontal overflow.

## I. Sticky Overlay Coexistence

Mini, Compact, and Expanded stay above Reader and outside Reader flow. Native trusted pointer input
moved Compact from top 76px to top 201px; the resulting rectangle intentionally overlapped the Reader
title, proving normal overlay behavior without reflow.

The first native wheel audit exposed a real WebView2 compositor issue: React's passive wheel listener
let the remaining delta scroll the underlying Reader during the same event that finished Todo's
internal scroll. The minimal compatibility fix attaches the Compact Todo wheel listener as native
`{ passive: false }`:

```text
Todo can still scroll:
  Todo 0 → 28; Reader 260 → 260

Todo is at boundary, next wheel event:
  Todo 28 → 28; Reader 260 → 350.4
```

No Todo length, persistence, clipping, or product behavior was changed.

## J. Appearance / Reader Preferences

The existing Appearance panel now contains one compact Reader section:

- font size: Small / Standard / Large;
- line spacing: Compact / Standard / Relaxed;
- defaults: Standard / Standard.

The controls are native buttons with `role=group`, labels, disabled saving state, and `aria-pressed`.
Persistence follows the existing boundary:

```text
React App state
→ PreferencesPort
→ Tauri update_preferences
→ tauri-plugin-store preferences.json
```

Native smoke selected Large / Relaxed, terminated the exact canonical process, restarted it with the
same isolated profile, and restored `large` / `relaxed` plus all four fixture Todos. `readerSkin=grid`
also restored. No Reader preference entered SQLite.

## K. Responsive Layout

| Preset | Logical target | Native viewport | Content width | Quote width | Code client / scroll | Horizontal overflow | Settings in viewport |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| Sticky | 320×420 | 320×420 | 309.6px | 273.6px | 273.6 / 418px | No | Yes |
| iPhone 5 | 320×568 | 320×568 | 309.6px | 264.8px | 264.8 / 418px | No | Yes |
| Pocket | 360×640 | 360×640 | 349.6px | 299.2px | 299.2 / 418px | No | Yes |
| Book | 420×594 | 420×595 | 409.6px | 350.8px | 350.8 / 418px | No | Yes |

At every size, native automation also opened Appearance, entered Mini, restored Compact, and changed
Reader scroll position to 120px. The 320×420 title begins at 162.4px and remains readable; code uses
internal horizontal scrolling rather than widening the page.

## L. Architecture Check

Architecture remains:

```text
React feature
→ application preference port
→ Tauri infrastructure / Store
```

`ReaderCanvas`, the fixture, and typography remain in `src/features/reader`. Reader preferences extend
the existing TypeScript/Rust preference wire contract with serde defaults, preserving legacy Store
compatibility. There is no Reader Rust repository, SQLite Reader data, network loader, or domain model.

Migration check: `src-tauri/migrations` has no diff. Versions 0001, 0002, and 0003 are frozen. The
isolated native database reported `[[1, success], [2, success], [3, success]]`.

## M. Automated Tests

Final local validation used Node 22.23.2, pnpm 11.16.0, Rust 1.97.1, and the Windows MSVC linker.

```text
pnpm format: PASS
pnpm typecheck: PASS
pnpm lint: PASS
pnpm test: PASS (42/42)
pnpm build: PASS
cargo fmt --check: PASS
cargo clippy --all-targets --all-features --locked -- -D warnings: PASS
cargo test --all-targets --all-features --locked: PASS (20/20)
pnpm tauri build --no-bundle -- --locked: PASS
```

Coverage includes Safe Area order, all required Markdown elements, raw HTML rejection, four presets,
320px containment, Reader/Mini/Compact layering, Todo wheel boundary behavior, Reader preference Port
saves and startup restoration, legacy Rust preference defaults, M1 long-Record save paths, and prior
Sticky regressions.

## N. Windows Native Smoke

The exact canonical release was tested in real Tauri/WebView2 at `http://tauri.localhost/`, using an
isolated SQLite database, Store, window state, and WebView2 profile. It was not Vite or a browser
substitute.

Verified sequence:

```text
launch exact canonical
→ load local Reader fixture and darker grid
→ create four fixture Todos through real Tauri/SQLite command boundary
→ iPhone 5 Compact → Mini → Compact
→ verify Mini/title safe gap
→ trusted drag Compact over Reader
→ switch and inspect all four window presets
→ scroll Reader to quote, code, and article end
→ wheel Compact Todo internally, then hand off at boundary
→ open Appearance, choose Large and Relaxed
→ terminate and restart exact canonical with same isolated data
→ restore preferences and four Todos
→ remain alive for at least 10 seconds
```

Result: **PASS**. Both canonical runs had empty stdout/stderr and no captured JS console error,
unhandled runtime exception, or WebView log error.

Product Owner AppData remained read-only. SHA-256 values before and after were identical:

| File | SHA-256 |
| --- | --- |
| `.window-state.json` | `BB25845431F97286176B1C0EA6BA369A82A91C87775DAA7E4F40AC0B8F497704` |
| `agent-desk.sqlite3` | `8C7F505E8997DF1B1EAB2755BED9A0DBE3735D90D9D874727C4AC0502C8EF031` |
| `preferences.json` | `8F30011D7FA91159488D7CAEEE17063E400CDBBC19F464F18F582FFF399A9D74` |

No Agent Desk process remained after the smoke.

## O. Windows/macOS CI

GitHub Actions run [31869807961](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31869807961)
ran against Final Handoff Commit `1fe815b7b6f1102d05d5aaa5c71a66e8e8680d77`.

| Job | Job ID | Result | Completed UTC |
| --- | ---: | --- | --- |
| [windows-latest](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31869807961/job/94976580251) | 94976580251 | **PASS** | 2026-08-15 06:41:52 |
| [macos-latest](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31869807961/job/94976580272) | 94976580272 | **PASS** | 2026-08-15 06:38:37 |

Both jobs truly completed, including frozen pnpm install, format, typecheck, lint, tests, Rust fmt,
Clippy, Rust tests, and `pnpm tauri build --no-bundle`. Workflow presence alone was not treated as
evidence.

## P. Product Preview Evidence

Every image below comes from canonical executable SHA-256
`A3A5D9B4C8D83599382D5DF94313D2CF402B0CF67A8A7F32C2B060E2D6704FD5` and implementation commit
`cf98307360f56d67e1902efa7ce35a107ed43a12`.

| Evidence | SHA-256 |
| --- | --- |
| [iPhone 5 + Mini](evidence/m2-a/windows/01-iphone5-mini.png) | `BABAB84F54CAED01AED529991BF3B0F55EB155C0A619B8B9A071425A3212E7AE` |
| [iPhone 5 + Compact overlap](evidence/m2-a/windows/02-iphone5-compact-overlap.png) | `1AFD463CC3B8BDC26416926B1EB469A53C780FBA48CFA07A4CC98C48EBFB33DF` |
| [Book Page](evidence/m2-a/windows/03-book-420x594.png) | `07DD4D29EC745C4F84732BCFB98598D67641E107AC7D296D62C0B7D136CE1C68` |
| [Blockquote](evidence/m2-a/windows/04-blockquote.png) | `2263C997A256D9AD2CEF93875B8E6839A23FA85CF02135D269ECC1C1B7D65A02` |
| [Code block](evidence/m2-a/windows/05-code-block.png) | `A0AE764B60E5556B60888F54FF9B3598EF651DDA6A4AE0E2CC7E6ECFC67BFB06` |
| [Long-scroll article end](evidence/m2-a/windows/06-long-scroll.png) | `8C078B74B88ADA0B7FBAF141E688883E19C6C0AAC76EAD89DFD1A93C38E189B6` |
| [Appearance Reader settings](evidence/m2-a/windows/07-reader-appearance-settings.png) | `AE34FECB2507C650B50BBCBB0B70C86E542E1A84E724EC77065EA5D4FDEC6663` |
| [Final darker grid after restart](evidence/m2-a/windows/08-final-darker-grid-after-restart.png) | `9E6F1F5DDCB63D0C9E28069F5EFC2A42F7D6B9497CFC9BFB288BC6FB2FA13146` |
| [Sticky 320×420](evidence/m2-a/windows/09-sticky-320x420.png) | `1D94A46F0BF08932E0B1BD26A60E2589ED497A92574A085BFE8178C7B7EAA639` |
| [Pocket 360×640](evidence/m2-a/windows/10-pocket-360x640.png) | `B5475FB1C73A98C9F72A51091E9C28386826F3B881C612F6974D43788D293086` |

Machine-readable measurements and assertions are in
[native-smoke.json](evidence/m2-a/windows/native-smoke.json), SHA-256
`BB05146969382F592C23B19FD2DB21F45E4FC4F9C1146A6F64A1BE6CF58F9723`.

## Q. Remaining Product Questions

For Product Owner review only:

1. 网格现在是否足够清楚？
2. Reader 是否像“一张桌面上的纸”？
3. 纸张质感是否舒服，还是过重？
4. 正文字号 / 行距默认值是否舒服？
5. 标题层级是否合适？
6. Blockquote 是否喜欢？
7. Code Block 是否喜欢？
8. Top Safe Area 是否自然？
9. Mini Tab 是否还会明显干扰第一行阅读？
10. 是否准备进入 M2-B？

No Kraft implementation is included in this review.

## R. Known Technical Issues

- WebView2 reports Book's requested 420×594 logical window as an inner viewport of 420×595 on this
  machine. Layout bounds, scrolling, Sticky, settings, quote, and code remain correct.
- Native screenshots reflect the machine's system-dark preference. Automated theme regression tests
  still cover System/Light/Dark behavior; M2-A did not alter the theme product decision.
- MSVC emits a localized informational `linker stdout` line while creating the import library. Build,
  Clippy, tests, and release linking all succeed; this is not a product/runtime warning.

No blocking product or technical issue remains in M2-A.

## S. Build Provenance

```text
Implementation Commit:
cf98307360f56d67e1902efa7ce35a107ed43a12

Final Handoff Commit:
1fe815b7b6f1102d05d5aaa5c71a66e8e8680d77

Frontend Bundle SHA-256:
dist/index.html
03C4F0A5777A524AD4B21090A8B606953BD6E9AA2F1DF0B2DEB1FE5B6E0A399A
dist/assets/index-4UYoVyXI.js
5142B9E58738FCB12B4B9555827B5E3B9C00CBB157DCC2E81F6622E3738B6424
dist/assets/index-ouhL1Zdw.css
7FD6266276BEC565941655E52A2BD069E73C0C4473528D87022971A3B830B9A8

Truth Build Executable:
D:\agent-desk-m2-a-final-verified-target\release\agent-desk.exe

Canonical Executable:
D:\agent-desk-target\release\agent-desk.exe

Canonical Executable SHA-256:
A3A5D9B4C8D83599382D5DF94313D2CF402B0CF67A8A7F32C2B060E2D6704FD5

Executable Size:
13,829,632 bytes

Executable Timestamp UTC:
2026-08-15T06:29:58Z

Product Preview Path:
D:\agent-desk-target\release\agent-desk.exe

Cargo.lock SHA-256:
075635A4168DA26E075DED6DD1B6C8A02F7E869D31CBE4950705D77A6E4D7DE5

pnpm-lock.yaml SHA-256:
EA90D425328B8BE2CF816932BD1291E6618D587C1F074A6CECAF8CCF309841DC

CI Run:
31869807961 — Windows PASS / macOS PASS
```

The verified target began as an absent directory and compiled the full locked Rust/Tauri graph. For
the final header-only CSS polish, its prior executable was explicitly removed before Tauri rebuilt and
relinked from the final production bundle; no old executable fallback was possible. Truth and
canonical hashes match exactly.

## T. Git State

```text
Branch: main
Implementation Commit: cf98307360f56d67e1902efa7ce35a107ed43a12
Final Handoff Commit: 1fe815b7b6f1102d05d5aaa5c71a66e8e8680d77
Origin: https://github.com/dawnsongbest-create/agent-desk.git
Remote branch: origin/main
Working tree at implementation/evidence handoff: clean
Gate report publication: docs-only commit containing this report
```

```text
M2_A_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
READER_CANVAS_STATUS: FOUNDATION_READY
```

Stop here. Do not enter M2-B.
