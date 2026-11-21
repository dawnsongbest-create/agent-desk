# M5-C ChatGPT 独立连接：技术与安全

ChatGPT Work Local → 插件包内 Agent Desk 可执行文件的 `--chatgpt-mcp` → IPv4 loopback Agent Bridge 的专用路由 → DeliveryService / ProposalService → SQLite → Inbox/Reader 或待确认建议。Codex STDIO、DuMate Streamable HTTP `/mcp` 和 OpenClaw 保留现有路由和凭据。

- 两个工具，`send_reading` 和 `suggest_todos`。成功只返回已提交或幂等回放收据，不暴露读取、文件或执行工具。待办建议不写正式 Todo；既有用户确认事务原子写入。正文保留 Markdown。阅读请求上限 1 MiB，建议上限 64 KiB、每批 1–20 条。
- `0012` 新增 `chatgpt_desktop_grants`，以 scope 为主键，摘要和最后成功调用时间分别存储；新授权默认不存在。扩展建议来源表的取值并复制旧批次全部字段、状态与回执，保留外键和唯一约束。ChatGPT 的历史批次和幂等命名空间在撤销/重新授权后不变。未编辑 0001–0011。
- 两项权限各有独立的高熵 OS credential；数据库保存派生的 HMAC 密钥摘要，不保存原始明文。**摘要对这个本机协议也是认证材料，生产数据库/备份必须作为敏感资产保护。** 包清单/参数不含任一凭据；隔离包携带非秘密的精确 profile 选择值，不自动回退到正式凭据。
- STDIO 工具读取同一 OS 用户的 Windows Credential Manager / macOS Keychain。仅在核对 Bridge 的 challenge HMAC 后发送正文；请求 HMAC 绑定 profile、端口、scope、随机 nonce 和正文摘要，服务端对 nonce 限时且一次性使用。仅 IPv4 loopback、严格 Host/Origin、无代理头、有限大小；无公网、无远程网关。授权变更、入站提交和停止通过同一 admission gate 串行；旧凭据不能穿越已完成的撤销。已落库内容不会回滚。
- 这里认证的是 **Agent Desk 签发的本机权限**，不是客户端声明、更不是 ChatGPT 进程代码签名。已获得同一系统账户凭据访问权的程序不在这个边界之外；不要共享系统账户、插件安装或数据库备份。撤权后重新授权意味着当前系统账户中还留着的同款安装包也可恢复使用；永久移除应同时卸载。
- ChatGPT-only 监听不会启用其他 Agent 的路由；原 Bridge 开关继续管理 OpenClaw/Codex/DuMate。ChatGPT 最后一项被撤销且其他 Agent 没有启用时，关闭监听。原生程序退出则全部连接不可用，重启按各自持久授权恢复。
- ZIP 只含两个便携清单和实际构建的当前平台可执行文件；启动命令使用包内 `./server/…` 相对路径，不设置不必要的工作目录。不复制系统凭据、插件缓存、用户数据库或日志。Release 构建 remap 开发路径；导出再次拦截常见个人编译路径。包由用户手动安装；不写 ChatGPT 隐藏配置。

## 回滚

先在 Agent Desk 撤销 ChatGPT，并卸载同名插件；保留历史数据。对安装候选升级应先备份停机 SQLite 及配套配置，并独立验证恢复。旧程序不应直接打开已迁移到 0012 的数据库；回退应用需使用迁移前的匹配备份。用户在新版本期间产生的数据不做自动删除、重建或回填。未批准前不覆盖 canonical、不升级正式数据库。
