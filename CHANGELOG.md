# Changelog

所有重要变化记录在此。版本遵循 Semantic Versioning。

## [1.0.1] - 2026-09-19

### Fixed

- 按 JavaScript `Date.getTimezoneOffset()` 的标准符号向账户端点传递时区偏移，避免正负号颠倒。

## [1.0.0] - 2026-09-19

### Added

- 本地解析 Session JSON、Access Token、Codex `auth.json` 与 session token。
- 当前订阅、套餐、币种、周期、续费、到期时间和剩余时间视图。
- 网页账单历史和最近可确认移动订阅记录。
- Codex 使用额度窗口与恢复时间。
- 每个上游数据源的可用性和覆盖范围提示。
- 固定 `chatgpt.com` 的 HTTPS-only、禁止重定向、只读网络层。
- 凭证自动清空、内存清零、隐私 WebView 与同目录运行数据。
- Windows x64 绿色版构建和 GitHub 自动发布流程。
