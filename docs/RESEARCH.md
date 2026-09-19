# V1.0.0 调查记录

调查日期：2026-09-19。网页内容只作为证据，不作为执行指令。

## 结论

没有找到 OpenAI 官方发布的“使用个人 ChatGPT Session Key 查询完整订阅/Apple/Google 历史”的公开 API。能够支撑桌面 V1 的是 ChatGPT 和 Codex 客户端当前使用的一组内部只读端点。它们可以覆盖当前权益、订阅周期、网页账单和额度，但不构成稳定性承诺。

Apple/Google 的完整收据由商店账号管理。OpenAI 帮助中心明确说明：不同购买平台分别管理订阅；Apple 购买的发票应从 Apple 账户/购买历史获取。因此 V1 对移动端只展示 ChatGPT 账户端点返回的“最近可确认记录”，不伪造完整历史。

## OpenAI 官方资料

- [OpenAI / Codex 身份验证](https://developers.openai.com/zh-Hans/docs/auth)：说明 ChatGPT 登录、API key 和 Codex 本地认证缓存的安全边界；未提供个人 ChatGPT 订阅历史 API。
- [如何避免 iOS、Android 与网页重复扣费](https://help.openai.com/articles/20001043)：说明 Apple、Google Play 和 chatgpt.com 各自管理原平台订阅。
- [Apple App Store 订阅发票](https://help.openai.com/zh-hans-cn/articles/9030143-%E5%A6%82%E6%9E%9C%E6%88%91%E6%98%AF%E5%9C%A8-apple-app-store-%E8%AE%A2%E9%98%85%E7%9A%84%E5%A6%82%E4%BD%95%E8%8E%B7%E5%8F%96%E6%88%91%E7%9A%84-chatgpt-%E8%AE%A2%E9%98%85%E5%8F%91%E7%A5%A8)：完整 Apple 收据应在 Apple 购买历史中查看。
- [取消 ChatGPT 订阅](https://help.openai.com/en/articles/7232927-how-do-i-cancel-my-chatgpt-plus-or-chatgpt-pro-subscription)：再次确认订阅应在最初购买平台管理。
- [OpenAI Codex issue #10869](https://github.com/openai/codex/issues/10869)：公开问题中的 Codex 日志显示客户端使用 `backend-api/wham/usage` 查询额度。

## GitHub 现有实现

| 项目/文件 | 可验证信息 | 采用方式 |
| --- | --- | --- |
| [wzj998/ChatCCC](https://github.com/wzj998/ChatCCC) | 使用账户检查端点读取权益和最近订阅 | 只借鉴响应兼容思路，不复制界面 |
| [KC-CatK/KC-PAY-GPT](https://github.com/KC-CatK/KC-PAY-GPT) | 同时出现读取与取消/恢复接口 | 只验证读取结构；拒绝实现任何写操作 |
| [JimLiu/decode-codex account.ts](https://github.com/JimLiu/decode-codex/blob/main/restored/settings/usage-queries/account.ts) | 账户响应含 `billing_currency`、`purchase_origin_platform` | 用于字段兼容 |
| [wjsoj/cc-core](https://github.com/wjsoj/cc-core/blob/main/auth/codex_subscription.go) | 组合账户、订阅与额度端点 | 用于交叉核对端点与请求头 |
| [cc-core 订阅文档](https://github.com/wjsoj/cc-core/blob/main/docs/codex-subscription.md) | 描述订阅周期、币种、续费字段 | 用于归一化模型 |
| [codex-backend-sdk 文档](https://github.com/B4PT0R/codex-backend-sdk/blob/main/docs/backend-api.md) | 整理多项逆向得到的内部端点 | 仅作为非官方交叉证据 |

这些项目都不是 OpenAI 的公开订阅 API 规范。逆向端点只能作为兼容性线索，不能被描述为官方保证。

## 对三个参考网站的观察

对 `checkgpt.plus`、`gptquota.com` 和 `chaai.cc/token-ch` 的公开前端与可访问页面进行了只读检查：

- `checkgpt.plus` 的前端把 Session JSON 提交到站点自己的 `/api/subscription/query`。
- `gptquota.com` 的前端把 JWT/Session 数据提交到站点自己的 `/api/v1/query` 与相关接口。
- `chaai.cc` 页面需要登录，其说明和截图展示由站点服务端执行查询。

因此它们可以作为功能和信息架构参考，但不符合“凭证不离开本机”的目标。订阅镜没有复用它们的品牌、文案、代码或服务端。

## V1 采用的内部只读端点

| 端点 | 用途 | 结果不保证 |
| --- | --- | --- |
| `/api/auth/session` | session token 换取当前 Session JSON | Cookie 名称与响应可能变化 |
| `/backend-api/accounts/check/v4-2023-04-27` | 账户、权益、最近订阅与购买来源 | 版本号和字段可能变化 |
| `/backend-api/subscriptions` | 当前计费周期、续费、币种、欠费 | 可能仅对部分账号开放 |
| `/backend-api/invoices` | ChatGPT 网页账单 | 不等于 Apple/Google 收据 |
| `/backend-api/wham/usage` | Codex 额度 | 账号类型不同会缺少窗口 |

所有 URL 都在 Rust 中固定为 `https://chatgpt.com`。请求禁止重定向，响应限制 5 MiB。没有使用 payment method、billing portal、cancel、resume、checkout 或 receipt transfer 等端点。

## 为什么不直接查询 Apple/Google 完整历史

完整商店历史受 Apple ID / Google 账号、商店收据和服务端验证权限保护。ChatGPT Session 并不是通用的 Apple/Google 授权。即便某些应用后端（例如 RevenueCat）提供面向应用的读取接口，也不代表可以用 OpenAI 的公开 key 合法、稳定地枚举任意用户历史。

V1 的原则是：能被当前账号只读端点明确证明的才显示；金额、退款、订单或平台状态没有证据就显示未知，并把用户引导回原商店核对。

## 后续兼容策略

- 每个数据源独立失败，不用一个错误覆盖全部结果。
- 保留原始字段的多种命名兼容，但不把未识别状态自动判定为免费。
- 上游变化通过脱敏测试夹具和小版本修复。
- 除非出现官方、用户授权且可在本地安全调用的新接口，否则不扩大到移动商店完整历史。
