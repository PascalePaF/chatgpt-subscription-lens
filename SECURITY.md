# 安全策略

## 报告漏洞

请使用 GitHub 仓库的私密安全报告功能，不要在公开 Issue 中粘贴 Session、Token、Cookie、邮箱、完整账户 ID、卡号或真实账单。

[创建私密安全报告](https://github.com/PascalePaF/chatgpt-subscription-lens/security/advisories/new)

## 支持范围

仅维护最新的 `1.0.x` 版本。ChatGPT 未公开端点变化造成的字段缺失可以作为兼容性问题报告，但请提供脱敏后的字段名与 HTTP 状态，不要提供原始响应。

## 设计限制

- 本地 JWT 检查只确认结构与到期时间，不执行 OpenAI 签名验证；
- 最终授权由 `chatgpt.com` 响应确认；
- Windows 剪贴板与进程内存不由本项目加密托管；
- 安装程序当前没有商业代码签名证书，Windows 可能显示 SmartScreen 提示；请从 GitHub Release 下载并核对 SHA-256。
