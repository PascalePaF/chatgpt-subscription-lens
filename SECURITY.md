# 安全政策

## 支持版本

| 版本 | 安全更新 |
| --- | --- |
| 1.1.x | 支持 |
| 1.0.x | 仅严重漏洞修复 |
| 更早版本 | 不支持 |

## 报告漏洞

请使用 GitHub 的 [私密安全报告](https://github.com/PascalePaF/chatgpt-subscription-lens/security/advisories/new)。不要在公开 Issue 中粘贴 Session JSON、Access Token、Cookie、邮箱、完整账号 ID 或真实账单。

报告中请包含：受影响版本、复现条件、预期/实际行为和最小化的脱敏示例。请不要测试不属于你的账号。

## 安全边界

这是只读查看器，不是凭证保管器。它不会：

- 长期保存、同步或恢复凭证；
- 代表用户购买、取消、续费、退款；
- 绕过 ChatGPT、Apple 或 Google 的身份校验；
- 保证内部端点长期可用。

如果怀疑凭证泄漏，请立即退出相关 ChatGPT 会话、修改账号安全设置，并按 OpenAI 官方支持渠道处理。
