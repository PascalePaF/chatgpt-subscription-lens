using System.Text;
using System.Text.Json;

namespace SubscriptionLens.Core;

internal sealed record JwtFacts(
    string Subject,
    DateTimeOffset ExpiresAt,
    string? Email,
    string? AccountId);

internal static class JwtInspector
{
    private const int MaxTokenLength = 32 * 1024;

    public static JwtFacts Inspect(string token, DateTimeOffset now)
    {
        if (string.IsNullOrWhiteSpace(token) || token.Length > MaxTokenLength)
        {
            throw new LensException("Access Token 长度异常。请重新复制完整凭证。");
        }

        var parts = token.Split('.');
        if (parts.Length != 3 || parts.Any(string.IsNullOrWhiteSpace))
        {
            throw new LensException("Access Token 不是完整的三段式 JWT。");
        }

        using var header = DecodePart(parts[0], "JWT 头部");
        using var payload = DecodePart(parts[1], "JWT 载荷");

        var algorithm = JsonAccess.String(header.RootElement, "alg");
        if (string.IsNullOrWhiteSpace(algorithm) || algorithm.Equals("none", StringComparison.OrdinalIgnoreCase))
        {
            throw new LensException("Access Token 的签名算法无效。");
        }

        var subject = JsonAccess.String(payload.RootElement, "sub");
        if (string.IsNullOrWhiteSpace(subject))
        {
            throw new LensException("Access Token 缺少 sub 身份字段，无法确认完整性。");
        }

        var expiresUnix = JsonAccess.Int64(payload.RootElement, "exp");
        if (expiresUnix is null)
        {
            throw new LensException("Access Token 缺少 exp 到期字段，无法确认完整性。");
        }

        DateTimeOffset expiresAt;
        try
        {
            expiresAt = DateTimeOffset.FromUnixTimeSeconds(expiresUnix.Value);
        }
        catch (ArgumentOutOfRangeException ex)
        {
            throw new LensException("Access Token 的到期时间无效。", ex);
        }

        if (expiresAt <= now.AddMinutes(-2))
        {
            throw new LensException($"Access Token 已于 {expiresAt.LocalDateTime:yyyy-MM-dd HH:mm:ss} 过期。");
        }

        var email = FirstNonEmpty(
            JsonAccess.String(payload.RootElement, "email"),
            JsonAccess.String(payload.RootElement, "https://api.openai.com/profile", "email"));

        var accountId = FirstNonEmpty(
            JsonAccess.String(payload.RootElement, "https://api.openai.com/auth", "chatgpt_account_id"),
            JsonAccess.String(payload.RootElement, "https://api.openai.com/auth/chatgpt_account_id"),
            JsonAccess.String(payload.RootElement, "chatgpt_account_id"));

        return new JwtFacts(subject, expiresAt, email, accountId);
    }

    private static JsonDocument DecodePart(string value, string fieldName)
    {
        if (value.Length > MaxTokenLength)
        {
            throw new LensException($"{fieldName}过长，凭证可能已损坏。");
        }

        try
        {
            var padded = value.Replace('-', '+').Replace('_', '/');
            padded += (padded.Length % 4) switch
            {
                2 => "==",
                3 => "=",
                0 => string.Empty,
                _ => throw new FormatException("Invalid Base64URL length"),
            };
            var bytes = Convert.FromBase64String(padded);
            return JsonDocument.Parse(bytes, new JsonDocumentOptions
            {
                AllowTrailingCommas = false,
                CommentHandling = JsonCommentHandling.Disallow,
                MaxDepth = 32,
            });
        }
        catch (Exception ex) when (ex is FormatException or JsonException)
        {
            throw new LensException($"{fieldName}无法解析，凭证可能没有复制完整。", ex);
        }
    }

    private static string? FirstNonEmpty(params string?[] values) =>
        values.FirstOrDefault(value => !string.IsNullOrWhiteSpace(value));
}
