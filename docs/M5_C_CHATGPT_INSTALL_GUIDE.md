# 把本机 ChatGPT 连到 Agent Desk

请使用带有 **ChatGPT** 连接卡的 Agent Desk，以及支持 **Work → Local** 的 ChatGPT 桌面端。网页版、移动端和云端会话不能启动这个安装包。安装过程不需要打开终端、复制 Token 或编辑配置。[OpenAI 桌面专用插件说明](https://developers.openai.com/plugins/deploy/desktop-only)

1. 在 Agent Desk 打开 **设置 → Agent 连接 → ChatGPT → 开始连接**。主动选择「阅读投递」或「待办建议」，也可以都选。确认后点击「保存安装包」。
2. 在同一台电脑、同一系统账户的 **ChatGPT Desktop → Work → Local → Plugins**，用实际提供的本地安装入口选择生成的 ZIP。不要上传到公共目录，也不要覆盖旧的 Agent Desk 预览插件。没有本地导入选项时，先停在这里，不要切换成远程连接。
3. 开一个新的 Local 会话，用 `@` 选择 **Agent Desk for ChatGPT**。第一次建议发一篇不含个人信息的测试文章或待办建议。Agent Desk 在收到有效请求之前会显示「已授权，等待首次调用」。

示例：「把下面这段 Markdown 作为《本地连接测试》发送到 Agent Desk。请真正调用工具，并给出回执，不要只生成示例。」阅读发送到 Inbox，可点击进入 Reader。建议先进入便利贴的「待确认建议」；你确认之前不会创建正式待办。

在 ChatGPT 卡片里可单独关闭阅读或待办权限、轮换凭据、或「断开 ChatGPT」。断开不删除已收到的文章和建议，也不影响其他 Agent。要彻底移除，再从 ChatGPT 插件界面卸载 **Agent Desk for ChatGPT**。

| 遇到的问题 | 先确认这些 |
| --- | --- |
| 无法导出安装包 | 至少授权一项权限；选择可写位置、新文件名。开发构建若包含个人路径，无法用于导出。 |
| 插件不能安装或找不到 | 从新的 Work **Local** 会话安装并检查插件权限；ZIP 仅适用于生成它的系统和架构。若界面只接受远程 URL，请停止。 |
| 连接失败 | 打开 Agent Desk，检查两项授权。若连接端口发生改变，在当前 Agent Desk 重新导出并安装。 |
| 权限拒绝 | 在 Agent Desk 开启**对应的一项**即可；不需重新复制凭据。 |
| 系统凭据异常 | Windows 检查 Credential Manager 是否可用；macOS 检查登录 Keychain 是否解锁，任何系统提示只在确认是当前安装的 Agent Desk 后处理。 |
| 不确定是否送达 | 先查 Inbox 或待确认建议。如果工具返回 `request_id`，仅用同一个请求 ID 和完全相同内容重试。 |

这是本地 Release Candidate 安装流程。macOS 客户端支持、系统签名/执行权限和 Keychain 提示需在目标机器单独核实；不要关闭 Gatekeeper 或绕过管理策略。
