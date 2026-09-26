# 订阅镜（Subscription Lens）

订阅镜是一款 **Windows 原生、本机运行、只读、开源** 的 ChatGPT 账户与订阅检查工具。它是 WPF 桌面应用，不包含 WebView，不启动本地网页服务器，也不会把凭证交给第三方查询站。

> 当前稳定开发线：`1.0.x`。项目在 2026-09-26 完成推倒重建；旧版代码仍保留在 Git 历史中，但不再作为现行实现。

## 能看到什么

- 当前套餐、是否有效、到期时间、剩余时间、是否续费、支付币种；
- 当前购买来源：ChatGPT 网页、Apple App Store 或 Google Play；
- 网页银行卡品牌与上游实际返回的脱敏位数；
- ChatGPT 网页端返回的最近账单；
- Codex 短周期、长周期及额外窗口的已用/剩余比例与重置时间；
- Chat、网页端 Pro、生图、Deep Research 的独立状态槽位。

最后四项中，只有 Codex 目前有可复现的独立只读额度接口。ChatGPT 网页端的生图和 Deep Research 剩余次数只会伴随真实会话事件出现；订阅镜不会为了“查额度”而发送消息或消耗次数，因此这些位置会诚实显示“当前接口未返回”。

## 不能承诺什么

- OpenAI 没有公开面向第三方的“个人 ChatGPT 订阅/账单 API”。除 Codex 额度外，本项目使用的 ChatGPT Web 账户端点都属于未公开接口，可能随时变更。
- 仅凭 ChatGPT Session 无法调用 Apple App Store Server API 或 Google Play Developer API 查询用户的完整商店历史。订阅镜只显示 ChatGPT 自身实际返回的当前购买来源。
- 支付方式接口通常只返回品牌、尾号和有效期，不保证返回卡号前 6 位。缺失时不会调用 BIN 服务、不会上传尾号、不会猜测。
- 本项目不恢复购买、不代充、不共享账号、不取消订阅、不退款、不创建 Stripe 客户门户，也不会把个人订阅包装成 API 服务。

完整证据与取舍见 [调研报告](docs/RESEARCH.md)。

## 安装

1. 打开 [GitHub Releases](https://github.com/PascalePaF/chatgpt-subscription-lens/releases/latest)。
2. 下载 `SubscriptionLens-v1.0.1-windows-x64-setup.exe`（V1.0.1 发布后）以及同名 `.sha256`。
3. 可选：用 PowerShell 核对哈希：

   ```powershell
   Get-FileHash .\SubscriptionLens-v1.0.1-windows-x64-setup.exe -Algorithm SHA256
   ```

4. 运行安装程序。默认安装目录：

   ```text
   %LOCALAPPDATA%\Programs\SubscriptionLens
   ```

这是按当前 Windows 用户安装的桌面软件，带开始菜单快捷方式与标准卸载程序；不是绿色免安装包。应用为 x64 自包含版本，不要求用户另装 .NET。

## 获取凭证

推荐顺序：

1. **完整 Session JSON**：登录自己的 `https://chatgpt.com/` 后，在同一浏览器配置中打开 `https://chatgpt.com/api/auth/session`，复制整个 JSON。
2. **Codex `auth.json`**：使用官方 Codex 登录后，复制完整文件内容。此方式的 Access Token 必须仍在有效期内。
3. **Access Token**：适合清楚令牌来源与风险的高级用户。
4. **Session Token / Cookie**：应用先用固定 Cookie 名尝试访问官方 Session 端点；只有官方返回完整 Session 后才会继续。

Session、Access Token 与 `auth.json` 都等同密码。不要发到 Issue、聊天群、截图或任何陌生网站。查询完成后，订阅镜会清空输入框；应用没有账户库、历史库或遥测模块。

## 界面原则

- 固定 1160 × 720 原生窗口，整个页面不滚动；
- 输入框固定约三行高，再长的 JSON 也不会撑满窗口；
- 套餐配色：Pro 20X 黑金、Pro 5X 蓝色、Plus 绿色、Free 灰色；
- 左侧显示套餐与有效期，右侧为黑色支付/商店卡片；
- 结果下半区同时容纳 Codex 额度、四类功能状态和账单记录；
- 端点部分失败时保留其他已确认数据，并给出简短警告。

## 从源码构建

要求：Windows 10/11 x64、.NET 8 SDK、NSIS 3。

```powershell
dotnet build .\SubscriptionLens.sln -c Release
dotnet run --project .\tests\SubscriptionLens.Tests\SubscriptionLens.Tests.csproj -c Release --no-build
.\scripts\build-release.ps1 -Version 1.0.1
```

构建输出：

- 自包含程序：`artifacts\publish\SubscriptionLens.exe`
- 安装程序：`release\SubscriptionLens-v1.0.1-windows-x64-setup.exe`
- 校验文件：同名 `.sha256`

## 安全模型

- 网络白名单只有 `https://chatgpt.com:443`；
- 所有业务请求均为 `GET`；
- 禁止 HTTP 重定向，避免 Authorization 头被转交；
- 单请求 20 秒超时，响应体有 2 MiB/8 MiB 上限；
- Session JSON 必须包含 `accessToken`、`user.email` 和可解析的 `expires`；
- JWT 必须是三段式、带非 `none` 算法、`sub`、`exp`，且尚未过期；
- 本地结构检查不等于密码学验签，最终授权仍由 `chatgpt.com` 判定；
- 不记录原始响应、Cookie、Token、完整账户 ID或邮箱日志。

安全问题请阅读 [SECURITY.md](SECURITY.md)；隐私说明见 [PRIVACY.md](PRIVACY.md)。

## 开源许可

MIT。详见 [LICENSE](LICENSE)。项目与 OpenAI、Apple、Google、Visa、Mastercard 无隶属或背书关系；产品名与商标归各自权利人所有。
