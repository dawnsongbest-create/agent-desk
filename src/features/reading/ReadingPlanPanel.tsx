import { useState, type FormEvent } from "react";
import type {
  CreateReadingPlanInput,
  ReadingDifficulty,
  ReadingPlan,
  ReadingPlanStatus,
} from "../../domain/reading";
import type { ReadingPlansState } from "./useReadingPlans";

const statusLabels: Record<ReadingPlanStatus, string> = {
  active: "进行中",
  paused: "已暂停",
  completed: "已完成",
};

export function ReadingPlanPanel({
  plans,
  state,
  error,
  busyPlanId,
  onRetry,
  onCreate,
  onGenerate,
  onSetStatus,
}: {
  plans: ReadingPlan[];
  state: ReadingPlansState;
  error: string | null;
  busyPlanId: string | null;
  onRetry(): void;
  onCreate(input: CreateReadingPlanInput): Promise<boolean>;
  onGenerate(id: string): Promise<boolean>;
  onSetStatus(id: string, status: ReadingPlanStatus): Promise<void>;
}) {
  const [creating, setCreating] = useState(false);
  const [title, setTitle] = useState("");
  const [sourceName, setSourceName] = useState("");
  const [dailyMinutes, setDailyMinutes] = useState(8);
  const [scheduleTime, setScheduleTime] = useState("08:00");
  const [difficulty, setDifficulty] = useState<ReadingDifficulty>("normal");
  const [contentMarkdown, setContentMarkdown] = useState("");

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const created = await onCreate({
      title,
      sourceName: sourceName || null,
      contentMarkdown,
      dailyMinutes,
      scheduleTime,
      difficulty,
    });
    if (!created) return;
    setTitle("");
    setSourceName("");
    setContentMarkdown("");
    setCreating(false);
  }

  return (
    <section className="reading-plan-panel" aria-label="阅读计划">
      <div className="reading-plan-heading">
        <div>
          <p>Reading Agent</p>
          <h2>阅读计划</h2>
        </div>
        <button type="button" onClick={() => setCreating((current) => !current)}>
          {creating ? "收起" : "+ 新计划"}
        </button>
      </div>

      {creating ? (
        <form className="reading-plan-form" onSubmit={(event) => void submit(event)}>
          <label>
            书名或计划名
            <input
              required
              maxLength={500}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </label>
          <label>
            来源名称
            <input
              maxLength={500}
              value={sourceName}
              placeholder="例如：本地 Markdown"
              onChange={(e) => setSourceName(e.target.value)}
            />
          </label>
          <div className="reading-plan-form-row">
            <label>
              每天分钟
              <input
                required
                type="number"
                min={1}
                max={240}
                value={dailyMinutes}
                onChange={(e) => setDailyMinutes(Number(e.target.value))}
              />
            </label>
            <label>
              推送时间
              <input
                required
                type="time"
                value={scheduleTime}
                onChange={(e) => setScheduleTime(e.target.value)}
              />
            </label>
            <label>
              内容难度
              <select
                value={difficulty}
                onChange={(e) => setDifficulty(e.target.value as ReadingDifficulty)}
              >
                <option value="normal">普通</option>
                <option value="technical">技术</option>
              </select>
            </label>
          </div>
          <label>
            Markdown 内容
            <textarea
              required
              value={contentMarkdown}
              placeholder="粘贴要分段阅读的 Markdown 正文…"
              onChange={(e) => setContentMarkdown(e.target.value)}
            />
          </label>
          <div className="reading-plan-form-actions">
            <span>内容仅保存在本机。</span>
            <button type="submit" disabled={state === "saving"}>
              {state === "saving" ? "创建中" : "创建阅读计划"}
            </button>
          </div>
        </form>
      ) : null}

      {error ? (
        <div className="reading-plan-error" role="alert">
          <span>{error}</span>
          <button type="button" onClick={onRetry}>
            重试
          </button>
        </div>
      ) : null}

      {state === "loading" && plans.length === 0 ? (
        <p className="reading-plan-empty">正在整理阅读计划…</p>
      ) : null}
      {state !== "loading" && plans.length === 0 && !creating ? (
        <p className="reading-plan-empty">创建第一份计划，让今天的内容安静地送到收件箱。</p>
      ) : null}

      <div className="reading-plan-list">
        {plans.map((plan) => {
          const busy = busyPlanId === plan.id;
          const exhausted = plan.currentOffset >= plan.totalContentLength;
          const progress = Math.min(
            100,
            Math.round((plan.currentOffset / plan.totalContentLength) * 100),
          );
          return (
            <article className="reading-plan-card" key={plan.id} data-status={plan.status}>
              <header>
                <div>
                  <span>{statusLabels[plan.status]}</span>
                  <h3>{plan.title}</h3>
                </div>
                <small>{progress}%</small>
              </header>
              <p>
                {plan.sourceName ?? "本地内容"} · 每天 {plan.dailyMinutes} 分钟 ·{" "}
                {plan.scheduleTime}
              </p>
              <div className="reading-plan-progress" aria-label={`阅读进度 ${progress}%`}>
                <span style={{ width: `${progress}%` }} />
              </div>
              <div className="reading-plan-actions">
                <button
                  type="button"
                  disabled={busy || plan.status !== "active" || exhausted}
                  onClick={() => void onGenerate(plan.id)}
                >
                  {busy ? "生成中" : exhausted ? "内容已送完" : "生成今日阅读"}
                </button>
                {plan.status === "active" ? (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void onSetStatus(plan.id, "paused")}
                  >
                    暂停
                  </button>
                ) : null}
                {plan.status === "paused" ? (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void onSetStatus(plan.id, "active")}
                  >
                    继续
                  </button>
                ) : null}
                {plan.status !== "completed" ? (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void onSetStatus(plan.id, "completed")}
                  >
                    完成阅读
                  </button>
                ) : null}
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}
