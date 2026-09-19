# 订阅镜 v1.1.0

一次完整的界面与交付方式升级。旧版 `v1.0.1` 标签和产物保持不变。

## 新界面

- 从零重做暖米色、陶土色点缀与衬线标题组成的编辑式桌面界面，不沿用参考站或旧版布局。
- 查询页、完整性校验、结果页、额度、账单和来源状态全部重新排版。
- Apple App Store、Google Play、Visa 与 Mastercard 使用本地内嵌单色品牌标识。

## 严格凭据检查

- 完整 Session JSON / Access Token 必须包含可解析 JWT、邮箱、用户 ID、ChatGPT 账户 ID 和未过期的有效期。
- Codex `auth.json` 同样检查账户范围与令牌声明。
- Session Token 先向 `chatgpt.com` 换取完整会话；验证完整后才读取订阅端点。
- 查询按钮在本地结构检查和账号归属确认完成前保持禁用。
- Rust 后端再次执行相同边界检查，不能绕过前端直接提交残缺凭证。

## 支付方式

- 新增只读 `/backend-api/payments/payment_methods` 数据源。
- 识别 Visa、Mastercard 及其他银行卡品牌。
- 只保留并显示上游实际返回的前 6 位和尾号 4 位；如果上游只返回尾号，会明确标注，不猜测前 6 位。
- iOS 与 Google Play 订阅显示对应商店标识，不把已保存的网页银行卡误认为移动订阅付款方式。

## 安装版

- Release 改为 Windows x64 NSIS 安装程序。
- 默认按当前用户安装，不要求管理员权限。
- 内嵌 WebView2 Bootstrapper 检查，提高首次安装兼容性。
- 不再发布绿色免安装 ZIP 或裸 EXE。

下载 `SubscriptionLens-v1.1.0-windows-x64-setup.exe`，校验值见 `SHA256SUMS.txt`。
