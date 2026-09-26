# 更新记录

## 1.0.0 — 2026-09-26

- 从空仓库内容重新实现，不沿用旧版界面与代码结构；
- 改为 .NET 8 WPF 原生 Windows 桌面软件，无 WebView；
- 固定单窗口、无页面滚动、三行高度凭证输入；
- 支持 Session JSON、Session Token/Cookie、Access Token、Codex auth.json；
- 严格本地完整性检查与远端 Session 二次核验；
- 只读查询账户、订阅、Codex 额度、网页账单和支付方式；
- 套餐四色与黑色支付/商店卡片；
- 端点部分失败不覆盖其他有效结果；
- 首批 26 项自动化测试与 Windows 安装程序。
