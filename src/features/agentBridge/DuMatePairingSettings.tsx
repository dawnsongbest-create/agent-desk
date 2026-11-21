import { useEffect, useRef, useState } from "react";
import {
  configureDuMate,
  copyDuMateAuthorization,
  getDuMateStatus,
  type DuMateScopes,
  type DuMateStatus,
} from "../../infrastructure/tauri/dumatePairing";

export function DuMatePairingSettings({ bridgeRunning }: { bridgeRunning: boolean }) {
  const [status, setStatus] = useState<DuMateStatus>();
  const [draft, setDraft] = useState<DuMateScopes>();
  const [working, setWorking] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const revision = useRef(0);
  const expiresAt = status?.expiresAt;
  const active = Boolean(status?.reading || status?.todo);

  useEffect(() => {
    const attempt = ++revision.current;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const load = () => {
      void getDuMateStatus()
        .then((result) => {
          if (revision.current === attempt) setStatus(result);
        })
        .catch(() => {
          if (revision.current === attempt) setError("无法读取百度搭子授权状态。");
        });
    };
    if (active && expiresAt) timer = setTimeout(load, Math.max(0, expiresAt * 1000 - Date.now()));
    load();
    return () => {
      revision.current += 1;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [bridgeRunning, active, expiresAt]);

  async function save(scopes: DuMateScopes, rotate: boolean) {
    revision.current += 1;
    setWorking(true);
    setError("");
    setNotice("");
    const requiresCopy =
      rotate || !active || (scopes.reading && !status?.reading) || (scopes.todo && !status?.todo);
    try {
      const updated = await configureDuMate(scopes, rotate);
      setStatus(updated);
      setDraft(undefined);
      setNotice(
        updated.reading || updated.todo
          ? requiresCopy
            ? "新凭据已生成，请重新复制并更新百度搭子配置。"
            : "已更新权限，有效期不变。"
          : "已撤销连接，旧凭据无法投递。",
      );
    } catch {
      setError("无法更新授权，请检查 Bridge 或系统凭据存储后重试。");
      try {
        setStatus(await getDuMateStatus());
      } catch {
        setStatus(undefined);
      }
    } finally {
      setWorking(false);
    }
  }

  async function copy() {
    setWorking(true);
    setError("");
    setNotice("");
    try {
      await copyDuMateAuthorization();
      setNotice("已复制 Authorization 的值。");
    } catch {
      setError("无法复制凭据。请检查授权是否到期，以及系统剪贴板权限。");
    } finally {
      setWorking(false);
    }
  }

  if (!status) return error ? <p role="alert">{error}</p> : null;

  const selected = draft ?? { reading: status.reading, todo: status.todo };
  return (
    <section className="agent-bridge-settings" aria-label="百度搭子">
      <strong>
        <span aria-hidden="true">▧ </span>百度搭子
      </strong>
      {!status.previewAvailable ? (
        <small>本地 MCP 仅在显式启用的隔离预览中可用，默认关闭。</small>
      ) : (
        <>
          <small>{status.bridgeRunning ? "Bridge 仅本机运行中" : "Bridge 已停止"}</small>
          <code className="agent-bridge-endpoint">{status.endpoint}</code>
          <p>
            阅读：{status.reading ? "已授权" : "未授权"} · 待办建议：
            {status.todo ? "已授权" : "未授权"}
          </p>
          {status.expiresAt ? (
            <small>
              {active ? "到期时间" : "已到期"}：
              <time dateTime={new Date(status.expiresAt * 1000).toISOString()}>
                {new Date(status.expiresAt * 1000).toLocaleString()}
              </time>
            </small>
          ) : null}
          <fieldset disabled={working || !bridgeRunning}>
            <legend>选择要授权的能力</legend>
            <label>
              <input
                type="checkbox"
                checked={selected.reading}
                onChange={(event) =>
                  setDraft({ ...selected, reading: event.currentTarget.checked })
                }
              />
              阅读投递
            </label>{" "}
            <label>
              <input
                type="checkbox"
                checked={selected.todo}
                onChange={(event) => setDraft({ ...selected, todo: event.currentTarget.checked })}
              />
              待办建议
            </label>
          </fieldset>
          <div className="agent-bridge-actions">
            <button
              type="button"
              disabled={
                working ||
                !bridgeRunning ||
                (selected.reading === status.reading && selected.todo === status.todo)
              }
              onClick={() => void save(selected, false)}
            >
              {active ? "保存权限" : "创建 30 分钟授权"}
            </button>
            <button
              type="button"
              disabled={working || !bridgeRunning || !active}
              onClick={() => void save({ reading: status.reading, todo: status.todo }, true)}
            >
              轮换凭据
            </button>
            <button
              type="button"
              disabled={working || !status.expiresAt}
              onClick={() => void save({ reading: false, todo: false }, false)}
            >
              撤销百度搭子
            </button>
          </div>
          <small>
            在百度搭子添加本地 Streamable HTTP 连接器，名称填 <code>Agent-Desk-DuMate</code>， URL
            使用上面的 MCP 地址，请求头名称填 <code>Authorization</code>。
          </small>
          <small>
            凭据仅用于 30
            分钟隔离预览。复制的值可能进入剪贴板历史或同步；填进百度搭子后无法保证仅保存在本机。
            请只用合成内容，测试后撤销授权并删除外部连接。
          </small>
          <div className="agent-bridge-actions">
            <button
              type="button"
              disabled={working || !bridgeRunning || !active}
              onClick={() => void copy()}
            >
              复制 Authorization 的值
            </button>
          </div>
          {notice ? <p role="status">{notice}</p> : null}
          {error ? <p role="alert">{error}</p> : null}
        </>
      )}
    </section>
  );
}
