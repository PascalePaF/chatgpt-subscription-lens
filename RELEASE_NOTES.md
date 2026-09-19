# 订阅镜 v1.0.0

首个稳定版本。

## 亮点

- 完全本地界面，没有作者中转服务器和遥测。
- 支持 Session JSON、Access Token、Codex `auth.json` 与 session token。
- 查看套餐、邮箱、当前状态、支付来源、币种、订阅周期、续费、到期/剩余时间。
- 整理 ChatGPT 网页账单、最近可确认移动订阅和 Codex 额度。
- 数据源逐项标记，明确区分“没有记录”与“本次端点不可用”。
- 单文件 Windows 可执行程序；运行数据只在解压目录内。

## 已知边界

OpenAI 没有公开个人 ChatGPT 完整订阅历史 API。Apple/Google 的完整历史必须在对应商店查询；本工具只显示 ChatGPT 账户端点最近可确认的移动订阅。内部端点可能随 ChatGPT 更新而变化。

下载 `SubscriptionLens-v1.0.0-windows-x64-portable.zip`，完整解压后运行 `SubscriptionLens.exe`。校验值见 `SHA256SUMS.txt`。
