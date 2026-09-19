# 贡献指南

感谢参与订阅镜。项目优先级依次是：不泄露凭证、只读、结果不误导、可复现构建、易用性。

## 本地检查

```powershell
npm ci
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## 提交要求

- 不得提交真实 Token、Session JSON、Cookie、邮箱或账单。
- 测试夹具必须完全虚构，JWT 只能使用不可登录的合成数据。
- 新增网络请求必须是读取类、固定 HTTPS 目标，并更新 `docs/RESEARCH.md` 和隐私说明。
- 不接受购买、退款、取消/恢复续费、收据转移、风控规避或批量账号查询功能。
- 对内部响应结构的兼容改动应同时添加单元测试。
- 界面不能把“端点不可用”显示成“免费/无历史”。

## Commit 建议

使用清晰的 Conventional Commits，例如 `feat: ...`、`fix: ...`、`docs: ...`、`test: ...`。
