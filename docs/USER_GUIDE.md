# 使用说明

## 第一次查询

1. 从 GitHub Releases 下载并安装订阅镜。
2. 登录自己的 `chatgpt.com`。
3. 推荐在同一浏览器配置中打开 `https://chatgpt.com/api/auth/session`，复制完整 JSON。
4. 粘贴到“会话凭证”框。输入区高度固定为约三行；内容仍然完整，只是在框内滚动。
5. 等待本地完整性提示变为绿色，勾选账户确认，点击“查询订阅”。
6. 查询成功后输入框会自动清空。

## 完整性检查到底检查什么

- Session JSON：必须有 `accessToken`、`user.email`、可解析且未过期的 `expires`；
- Codex auth.json：必须有 `tokens.access_token`；
- Access Token：必须是完整三段 JWT，包含有效 `alg`、`sub` 和未过期 `exp`；
- Session Token：必须达到合理长度且没有空格、换行和控制字符；随后还要由 ChatGPT Session 端点换取完整会话。

本地无法验证 JWT 的服务器签名；最终是否有权访问由 `chatgpt.com` 决定。

## 如何理解结果

- **Pro 20X / Pro 5X / Plus / Free**：来自账户或订阅响应；无法识别的新套餐会显示上游原始名称的安全格式。
- **剩余时间**：由服务返回的到期时间减去本机当前时间，不代表保证续费成功。
- **银行卡**：只显示服务实际返回的字段。通常是品牌、尾号和有效期；没有前 6 位时不会补全。
- **Apple / Google**：表示当前购买来源。完整订单和退款历史必须去对应商店查看。
- **Codex 额度**：显示短周期和长周期；“剩余”由 `100 - used_percent` 得出。
- **Chat / 生图 / Deep Research**：若只读接口未给出实时余量，就显示“未返回”，不是 0。
- **网页账单**：只属于 ChatGPT Web/Stripe，不包括 Apple 或 Google 收据。

## 常见错误

### “Session JSON 缺少 user.email”

复制的可能只是 Token 或截断片段。请重新打开 `/api/auth/session` 并复制整个 JSON。

### “Access Token 已过期”

重新登录 ChatGPT，或让官方 Codex 刷新登录后再复制新的 `auth.json`。

### 某几项显示不可用，但套餐仍显示

内部端点会独立失败，移动订阅尤其可能没有网页银行卡或 Stripe 发票。这不是应用把数据删掉了；已成功读取的项目仍会保留。

### 额度显示有剩余但 Codex 拒绝请求

服务端的执行 gate 与额度展示曾有公开不一致案例。订阅镜展示的是查询时返回的快照，不保证随后每个请求都会被接受。

## 卸载

在 Windows“已安装的应用”中选择“订阅镜”，或从开始菜单运行“卸载订阅镜”。应用默认不创建账户数据目录，因此卸载只移除安装文件、快捷方式和卸载项。
