import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import type { ReadingPlan } from "../../domain/reading";
import { ReadingPlanPanel } from "./ReadingPlanPanel";

const plan: ReadingPlan = {
  id: "plan-test",
  title: "设计系统阅读",
  sourceName: "本地 Markdown",
  contentMarkdown: "# 第一章\n\n正文",
  totalContentLength: 1000,
  dailyMinutes: 8,
  scheduleTime: "08:00",
  difficulty: "technical",
  status: "active",
  currentOffset: 250,
  currentDay: 1,
  createdAt: "2026-08-21T00:00:00Z",
  updatedAt: "2026-08-21T00:00:00Z",
};

function renderPanel(overrides: Partial<ComponentProps<typeof ReadingPlanPanel>> = {}) {
  const props: ComponentProps<typeof ReadingPlanPanel> = {
    plans: [plan],
    state: "ready",
    error: null,
    busyPlanId: null,
    onRetry: vi.fn(),
    onCreate: vi.fn(async () => true),
    onGenerate: vi.fn(async () => true),
    onSetStatus: vi.fn(async () => undefined),
    ...overrides,
  };
  return { ...render(<ReadingPlanPanel {...props} />), props };
}

describe("ReadingPlanPanel", () => {
  it("creates the local Markdown plan with the locked MVP fields", async () => {
    const user = userEvent.setup();
    const { props } = renderPanel({ plans: [] });
    await user.click(screen.getByRole("button", { name: "+ 新计划" }));
    await user.type(screen.getByLabelText("书名或计划名"), "产品阅读");
    await user.type(screen.getByLabelText("来源名称"), "本地资料");
    await user.clear(screen.getByLabelText("每天分钟"));
    await user.type(screen.getByLabelText("每天分钟"), "12");
    await user.type(screen.getByLabelText("Markdown 内容"), "# 第一章\n\n完整正文");
    await user.selectOptions(screen.getByLabelText("内容难度"), "technical");
    await user.click(screen.getByRole("button", { name: "创建阅读计划" }));
    expect(props.onCreate).toHaveBeenCalledWith({
      title: "产品阅读",
      sourceName: "本地资料",
      contentMarkdown: "# 第一章\n\n完整正文",
      dailyMinutes: 12,
      scheduleTime: "08:00",
      difficulty: "technical",
    });
  });

  it("generates today and supports pause and completion without redesigning Inbox", async () => {
    const user = userEvent.setup();
    const { props } = renderPanel();
    expect(screen.getByText("25%")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "生成今日阅读" }));
    await user.click(screen.getByRole("button", { name: "暂停" }));
    await user.click(screen.getByRole("button", { name: "完成阅读" }));
    expect(props.onGenerate).toHaveBeenCalledWith(plan.id);
    expect(props.onSetStatus).toHaveBeenNthCalledWith(1, plan.id, "paused");
    expect(props.onSetStatus).toHaveBeenNthCalledWith(2, plan.id, "completed");
  });
});
