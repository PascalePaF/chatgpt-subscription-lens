use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("没有检测到可用凭证。请粘贴 Session JSON、Access Token 或 Codex auth.json。")]
    MissingCredential,
    #[error("凭证内容超过 1 MiB 安全上限，已拒绝解析。")]
    CredentialTooLarge,
    #[error("凭证格式无法识别。建议复制 https://chatgpt.com/api/auth/session 页面中的完整 JSON。")]
    UnsupportedCredential,
    #[error("Session cookie 含有不安全字符，已拒绝发送。")]
    UnsafeSessionToken,
    #[error("无法从 Session cookie 获取 Access Token：{0}")]
    SessionExchange(String),
    #[error("Access Token 已过期，请重新登录 ChatGPT 后获取新的 Session JSON。")]
    ExpiredToken,
    #[error("无法创建本地网络客户端：{0}")]
    Client(String),
    #[error("没有任何订阅数据源查询成功：{0}")]
    NoSources(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
