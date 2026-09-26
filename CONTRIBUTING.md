# 贡献指南

欢迎提交兼容性修复、测试、无障碍改进和文档补充。

## 基本要求

- 不提交真实 Session、Token、Cookie、账户响应或账单；
- 网络范围保持 `chatgpt.com:443`、GET-only、无重定向；
- 不加入恢复购买、取消、退款、绑卡、代充或代理转发；
- 未公开字段必须有捕获或多源证据，并提供脱敏测试夹具；
- 缺失值必须显示未知，不得用套餐宣传值冒充实时值；
- UI 不引入 WebView，也不加入页面级滚动。

## 提交前

```powershell
dotnet build .\SubscriptionLens.sln -c Release
dotnet run --project .\tests\SubscriptionLens.Tests\SubscriptionLens.Tests.csproj -c Release --no-build
```

PR 请说明：变化范围、验证方式、是否涉及凭证或网络边界、脱敏夹具来源。
