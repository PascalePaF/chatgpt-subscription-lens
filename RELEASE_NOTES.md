# 订阅镜 v1.1.1

V1.1.1 把整个呈现层从 WebView/Tauri 重建为真正的 Windows 原生桌面程序，并按用户反馈重新组织为固定单屏的订阅卡片。

## 原生桌面程序

- 移除 TypeScript、Vite、Tauri 与 WebView2；EXE 不再承载网页界面。
- 使用 Rust、eframe/egui、winit 和本机 OpenGL 绘制固定 1280 × 800 窗口。
- 安装目录不再创建 `subscription-lens-data/WebView2`，运行时没有 WebView 子进程。
- 保留标准 Setup EXE、开始菜单入口、Windows 卸载项与静默卸载支持。

## 全新单屏界面

- Claude 风格的暖白、炭黑与陶土色界面，左侧窄导航，取消页面级滚动。
- 套餐颜色：Pro 20X 黑金、Pro 5X 蓝色、Plus 绿色、Free 灰色。
- 卡片正面左侧展示当前套餐、剩余时间和到期时间；右侧使用高级黑色支付卡片展示渠道。
- 卡片背面左侧集中展示账户/订阅与 Codex、Chat、网页端 Pro、生图、Deep Research 五类额度；右侧展示最近 4 条账单。
- 删除界面中的“数据来源”区。无法验证的数据明确显示“未返回”。

## 输入与支付方式

- Session 输入框固定只显示三行，不再占据大块页面。
- 完整性检查仍在查询前和 Rust 查询入口各执行一次；session token 在访问其他端点前必须换取完整身份与账户范围。
- 新增四枚随 EXE 内嵌的透明背景 3D 图标：Visa、Mastercard、Apple App Store、Google Play。
- 银行卡继续严格限制为实际返回的前 6 位和尾号 4 位，中间始终隐藏。

## 验证

- 14 项 Rust 单元测试全部通过。
- `cargo fmt` 与 `cargo clippy -D warnings` 通过。
- 已执行安装、原生窗口启动、零 WebView 数据/子进程检查和卸载回归。

下载 `SubscriptionLens-v1.1.1-windows-x64-setup.exe`，校验值见同一 Release 的 `SHA256SUMS.txt`。
