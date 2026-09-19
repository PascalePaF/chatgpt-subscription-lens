# Changelog

所有重要变化记录在此。版本遵循 Semantic Versioning。

## [1.1.0] - 2026-09-19

### Added

- 查询前的凭据结构、身份、账户范围和有效期检查；Rust 后端强制执行完整性门槛。
- 只读支付方式数据源及 Visa、Mastercard、Apple App Store、Google Play 本地标识。
- 银行卡显示最多保留前 6 位和尾号 4 位；未返回的字段不会猜测。
- Windows x64 NSIS 安装程序、开始菜单入口和标准卸载流程。

### Changed

- 从零重做为暖米色、陶土色点缀与衬线标题组成的编辑式桌面界面。
- GitHub Release 仅发布 Setup EXE，不再生成绿色免安装包。

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
