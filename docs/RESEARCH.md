# ChatGPT 订阅本地查询调研报告

调研日期：2026-09-26

目的：判断“仅在本机使用用户自行提供的 ChatGPT 会话，查看订阅、支付来源、网页账单和额度”哪些部分有官方依据，哪些只能依赖未公开接口，以及哪些承诺在技术上不成立。

## 结论先行

1. OpenAI 没有公开供第三方桌面应用读取个人 ChatGPT 订阅、银行卡或账单的正式 API。
2. 官方 Codex 客户端公开源码确认了 ChatGPT 订阅登录、`auth.json` 缓存、`ChatGPT-Account-ID` 以及 `/backend-api/wham/usage` 额度读取；这一部分证据最强。
3. `/api/auth/session`、`accounts/check`、`subscriptions`、`invoices` 和 `payment_methods` 是 ChatGPT Web 的内部只读接口。开源项目和实际捕获可以交叉证明，但 OpenAI 没有保证其兼容性。
4. Apple/Google 的开发者订阅 API都要求应用开发者凭证以及该应用的交易标识或 purchase token。ChatGPT Session 不能替代这些权限，也不能获取用户完整的 App Store/Google Play 历史。
5. ChatGPT 网页端的 Deep Research、生图等剩余计数，没有独立只读 GET 接口。社区捕获显示它们随真实会话的 SSE 元数据出现。为了查询而创建会话会消耗次数、改变账户状态，因此本项目明确不做。

## 证据等级

| 等级 | 含义 | 本项目如何使用 |
|---|---|---|
| A | 官方文档明确支持 | 可作为稳定产品能力，但仍保护凭证 |
| B | 官方开源客户端实现 | 按客户端契约实现并跟踪上游变化 |
| C | 未公开接口，有多个开源实现/捕获交叉验证 | 标记实验性、逐接口容错、不得据此保证长期可用 |
| D | 社区反馈或单次观察 | 只用于需求和风险设计，不当作协议保证 |
| E | 高风险、写操作或灰色用途 | 排除，不实现 |

## OpenAI 官方资料

### 登录与凭证（A）

- [Codex Authentication](https://learn.chatgpt.com/docs/auth) 明确区分“使用 ChatGPT 订阅登录”与“使用 API Key 按量计费”。浏览器完成登录后，Codex 会缓存登录状态并自动刷新令牌。
- 同一文档说明 `~/.codex/auth.json` 或系统凭证存储包含访问令牌，应像密码一样保护，不应提交仓库或分享。
- [Codex access tokens](https://learn.chatgpt.com/docs/enterprise/access-tokens) 将 Codex 访问令牌定义为 ChatGPT 工作区凭证，强调可信本地工作流、最小权限、轮换与撤销。
- OpenAI API 文档中的 ChatKit Session、Realtime Session 等“session”是应用 API 对象，不是 `chatgpt.com` 浏览器登录 Session，不能混用。

结论：官方支持用户在本机通过 ChatGPT 登录使用 Codex，也正式承认缓存访问令牌；官方没有把浏览器 Session 定义为第三方订阅查询 API。

### 官方 Codex 源码（B）

- [openai/codex](https://github.com/openai/codex) 的 `backend-client` 将 ChatGPT 风格的额度地址映射到 `/wham/usage`，基地址为 `https://chatgpt.com/backend-api`。
- 源码读取 `ChatGPT-Account-ID`，解析短周期、长周期、额外额度窗口、后端 `allowed` 判定、套餐类型和重置时间。
- 官方实现明确指出：展示百分比不等同于后端实际允许推理；`allowed` 才是执行判定。这直接影响本项目的状态文案。

## Apple 与 Google 官方资料（A）

- [Apple：在 iPhone 上查看购买和订阅](https://support.apple.com/guide/iphone/see-your-purchases-and-subscriptions-iph4e3e7324f/ios) 说明完整历史由用户在自己的 App Store 账户中查看。
- [App Store Server API](https://developer.apple.com/documentation/appstoreserverapi) 需要开发者授权 JWT 和交易标识，只能读取该开发者应用的顾客交易。
- [Google Play：查看订单历史](https://support.google.com/googleplay/answer/2850369) 说明用户在 Play Store、play.google.com 或 payments.google.com 查看订单。
- [Google Play Developer API：purchases.subscriptionsv2.get](https://developers.google.com/android-publisher/api-ref/rest/v3/purchases.subscriptionsv2/get) 需要 Android Publisher OAuth 权限、packageName 和 purchase token。

因此，订阅镜能显示的移动端信息上限是：ChatGPT 自身账户响应明确返回的当前购买平台、到期与续费状态。它不会声称能从 Session 还原完整 Apple/Google 收据历史。

## GitHub 开源项目横向调研

以下项目用于理解实现方式与用户需求，不复制其界面，也不盲目信任逆向字段。

| 项目 | 观察到的做法 | 对订阅镜的启示 |
|---|---|---|
| [openai/codex](https://github.com/openai/codex) | 官方 Rust 客户端；ChatGPT OAuth、`auth.json`、app-server、`wham/usage` | 额度模型和请求头的首要依据 |
| [joningi/chatgpt-export](https://github.com/joningi/chatgpt-export) | Session Cookie 换取 `/api/auth/session` 的 Bearer；强调 Cookie 等同密码；只读 GET | Session 兼容入口必须二次核验且不得存储 |
| [bcharleson/codexbar](https://github.com/bcharleson/codexbar) | 通过 Codex app-server 的 `account/read`、`account/rateLimits/read` 读取额度 | 官方本地进程是理想方向，但不应成为独立 EXE 的强制依赖 |
| [Duoasa/QuotaView](https://github.com/Duoasa/QuotaView) | 原生 macOS UI，通过本机 Codex app-server 获取额度，不直接读取凭证 | 证明“原生、小窗口、明确重置时间”有真实需求 |
| [wenzetan/dsh-quota-panel](https://github.com/wenzetan/dsh-quota-panel) | 使用 `/backend-api/wham/usage`，明确警告响应形状会变化 | 必须容忍未知字段与部分失败 |
| [V1ki/dsh-plugin-subscriptions](https://github.com/V1ki/dsh-plugin-subscriptions) | OAuth 订阅接入与 `wham/usage` | 订阅 OAuth 与 Platform API Key 是两类计费体系 |
| [eduardopessin/tokengateway](https://github.com/eduardopessin/tokengateway) | 令牌管理和额度面板 | 凭证生命周期与额度展示应分层 |
| [k7631159/ai-fuelgauge](https://github.com/k7631159/ai-fuelgauge) | CLI/托盘/HUD 额度监控 | 用户重视倒计时与窗口差异 |
| [B4PT0R/codex-backend-sdk](https://github.com/B4PT0R/codex-backend-sdk) | 非官方 SDK，整理 Codex/WHAM 路由 | 仅作 B/C 之间的交叉证据，不视为 OpenAI 文档 |
| [wjsoj/cc-core 订阅说明](https://github.com/wjsoj/cc-core/blob/main/docs/codex-subscription.md) | 捕获 `subscriptions`、`accounts/check`、`payment_methods` 与 invoice 路径，强调部分成功 | 每个端点独立失败，不能用一个 401 抹掉全部结果 |
| [walter1297/ChatGPTSub](https://github.com/walter1297/ChatGPTSub) | 涉及恢复购买/收据流程 | 属于写操作和高风险边界，本项目明确排除 |

调研时还遇到大量注册机、代充、绑卡、共享账户、代理转发仓库。它们不属于本项目目标，也不作为可接受实现来源。

## 可复现的只读接口矩阵

| 路径 | 方法 | 预期信息 | 证据 | 风险与处理 |
|---|---|---|---|---|
| `/api/auth/session` | GET | 浏览器 Session 换取完整 Session JSON | C | Cookie 名和字段会变化；返回缺字段立即停止 |
| `/backend-api/accounts/check/v4-2023-04-27` | GET | 账户、entitlement、购买来源、最近订阅 | C | 同一账户可能以 UUID 与 `default` 重复；按明确优先级选择 |
| `/backend-api/subscriptions?account_id=` | GET | 本期起止、周期、币种、续费、欠款 | C | `account_id` 必填；HTTP 200 也可能是错误信封 |
| `/backend-api/wham/usage` | GET | Codex 短/长周期、额外窗口、允许状态 | B | 百分比与实际 gate 可能短时不一致；保留后端状态 |
| `/backend-api/invoices?limit=12&account_id=` | GET | ChatGPT Web 的 Stripe 发票 | C | 不等于 Apple/Google 收据；响应较大，单独限 8 MiB |
| `/backend-api/payments/payment_methods?account_id=` | GET | 卡品牌、尾号、有效期 | C | 移动订阅可返回 400；通常没有前 6 位 |

所有路径固定到 `https://chatgpt.com:443`，业务方法仅 GET，禁止重定向。`customer_portal`、checkout、restore、cancel、refund 等路径不在白名单内。

## 社区反馈与需求

### 反复出现的需求

- 同时看到短周期和长周期，而不是一个模糊“剩余百分比”；
- 显示精确重置时刻和剩余倒计时；
- 清楚区分 Codex 订阅额度、ChatGPT 网页功能额度与 Platform API 计费；
- 显示数据新鲜度、未知状态和部分失败，而不是把缺失字段当作 0；
- 移动内购要明确“由 Apple/Google 管理”；
- 凭证不上传、无遥测、无需常驻服务。

### 已公开的不一致案例（D）

- [openai/codex#39850](https://github.com/openai/codex/issues/39850)：一个 Bearer 可继续访问额度，但账户设置返回 401；说明不同端点会独立失败。
- [openai/codex#30970](https://github.com/openai/codex/issues/30970)：客户端显示 Pro 和 100% 余量，但推理 gate 按 Free 阻止。
- [openai/codex#36344](https://github.com/openai/codex/issues/36344)：面板仍有余量而请求被限流。
- [openai/codex#13186](https://github.com/openai/codex/issues/13186)：小任务消耗与短/周窗口显示出现争议。
- [openai/codex#16292](https://github.com/openai/codex/issues/16292)：工作区停用导致多个后端接口统一返回 402。

产品决定：端点逐项显示成功/不可用；只要核心项有一项成功就保留结果；不把“100% 剩余”写成“保证可调用”。

## 为什么不显示虚构的 Chat/生图/Deep Research 次数

社区对 ChatGPT Web 的捕获显示，`accounts/check` 提供套餐事实但不提供剩余计数；尝试过的 `/feature_limits`、`/limits`、`/rate_limits` 等独立路径返回 404。Deep Research 与 `image_gen` 的 `remaining` 出现在真实会话的 `conversation_detail_metadata.limits_progress` 流事件里。

读取这些值需要发起一次对话或依赖过去会话的本地缓存。前者会改变用户数据并可能消耗额度，后者不能代表当前状态。V1.0.x 选择显示“未返回实时余量”，这比伪造套餐固定次数更可靠。

## V1.0.0 架构决定

- WPF + .NET 8 自包含 x64：真正桌面窗口，无 WebView；
- 两个固定状态：连接页和结果页，窗口内无页面级滚动；
- 输入框固定三行高度，长 JSON 只在框内滚动；
- Session JSON、Codex auth.json、Access Token 在本地严格解析；Session Token 先换取完整会话；
- 账户端点先读取并解析真实 account_id，其余只读端点并行；
- 每个响应独立状态，未知字段被忽略，必需字段缺失则该项不可用；
- 不落盘、不写日志、不保存历史；
- 26 项初始测试覆盖凭证、JWT、多账户选择、套餐映射、商店来源、卡号脱敏、账单金额、额度窗口和 GET-only 网络契约。

## 明确排除（E）

- 自动读取浏览器 Cookie/密码库；
- 上传 Session 到本项目服务器或第三方；
- 收据重放、恢复购买、代充、共享账户；
- 取消/退款/改卡/创建客户门户；
- 将个人订阅转成对外 API；
- 通过 BIN 查询补全卡号信息；
- 为获取额度而自动发送 ChatGPT 消息。

## 后续观察点

- OpenAI 是否发布个人 ChatGPT 订阅的正式只读 API；
- 官方 Codex app-server 的账户/额度接口是否适合成为可选来源；
- 内部端点字段或 Cookie 名变更；
- WHAM 多额度窗口与 `allowed` 字段的新形状；
- 对长邮箱、200% DPI、小屏任务栏和无障碍键盘路径的持续测试。

## V1.0.1 自审结果

V1.0.0 发布后以实际安装包为基线进行了第二轮检查，发现并修复：固定窗口在高 DPI 逻辑工作区下可能超界、HTTP 200 错误对象会被计为成功、单文件自包含会把原生组件解压到临时目录、根级默认银行卡 ID 未参与选择，以及 JWT 极端时间字段未完全归类为用户可读错误。V1.0.1 增加相应自动化夹具，并将整个 1160 × 680 设计画布在较小工作区内等比缩放。

最终验证包括 43 项离线自动化测试、全规则静态分析、格式与差异检查，以及真实安装包的安装—卸载—重装和已安装程序自检。
