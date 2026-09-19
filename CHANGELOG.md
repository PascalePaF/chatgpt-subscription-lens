# Changelog

所有重要变化记录在此。版本遵循 Semantic Versioning。

## [1.1.1] - 2026-09-19

### Added

- 固定 1280 × 800 的 Rust 原生 Windows 窗口与单屏卡片正反面布局。
- Pro 20X 黑金、Pro 5X 蓝色、Plus 绿色、Free 灰色套餐配色。
- Visa、Mastercard、Apple App Store、Google Play 四枚本地内嵌 3D 图标。
- Codex、Chat、网页端 Pro、生图、Deep Research 五类固定额度槽位；没有可验证字段时明确显示“未返回”。
- 自定义 NSIS 当前用户安装器、开始菜单入口、卸载项与版本资源。
- 从 V1.1.0 中文目录自动迁移到新的当前用户安装目录。

### Changed

- 完整移除 Tauri、TypeScript、Vite、HTML/CSS/JavaScript 和 WebView2 运行路径。
- Session 输入区缩为三行；卡片正面集中显示订阅与支付方式，背面集中显示额度和最近账单。
- Release 构建改为 Cargo 原生编译加项目内 NSIS 脚本。

### Removed

- 页面级纵向滚动与“数据来源”界面。
- WebView2 数据目录、浏览器内核子进程和旧网页前端资源。

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
- 凭证自动清空、内存清零与当时版本的隐私 WebView 运行模式。
- Windows x64 绿色版构建和 GitHub 自动发布流程。
