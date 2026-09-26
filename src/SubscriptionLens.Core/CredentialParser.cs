using System.Text.Json;
using System.Text.RegularExpressions;

namespace SubscriptionLens.Core;

public static partial class CredentialParser
{
    private const int MaxCredentialCharacters = 512 * 1024;
    private const int MinOpaqueSessionLength = 48;
    private const int MaxOpaqueSessionLength = 16 * 1024;

    private static readonly string[] SupportedCookieNames =
    [
        "__Secure-next-auth.session-token",
        "__Secure-authjs.session-token",
        "oai-client-auth-session",
    ];

    public static CredentialValidation Validate(string? input, DateTimeOffset? now = null)
    {
        try
        {
            using var material = Parse(input, now ?? DateTimeOffset.UtcNow);
            var title = material.Kind switch
            {
                CredentialKind.SessionJson => "完整 Session JSON",
                CredentialKind.CodexAuthJson => "完整 Codex auth.json",
                CredentialKind.AccessToken => "完整 Access Token",
                CredentialKind.CookieHeader => "会话 Cookie",
                CredentialKind.SessionToken => "Session Token",
                _ => "凭证",
            };
            var detail = material.NeedsSessionExchange
                ? "本地结构检查通过；查询时还会向 chatgpt.com 核验并换取完整会话。"
                : $"本地结构检查通过；令牌有效至 {material.ExpiresAt?.LocalDateTime:yyyy-MM-dd HH:mm:ss}。";
            return new CredentialValidation(true, material.Kind, title, detail, material.NeedsSessionExchange);
        }
        catch (LensException ex)
        {
            return new CredentialValidation(false, null, "凭证不完整", ex.Message, false);
        }
    }

    public static CredentialMaterial Parse(string? input, DateTimeOffset now)
    {
        var value = Normalize(input);

        if (value.StartsWith('{') || value.StartsWith('['))
        {
            return ParseJson(value, now);
        }

        if (LooksLikeJwt(value))
        {
            return FromAccessToken(value, CredentialKind.AccessToken, null, null, now);
        }

        if (TryExtractCookieHeader(value, out var cookieHeader, out var sessionToken))
        {
            ValidateOpaqueSessionToken(sessionToken);
            return new CredentialMaterial
            {
                Kind = CredentialKind.CookieHeader,
                CookieHeader = cookieHeader,
                SessionToken = sessionToken,
            };
        }

        ValidateOpaqueSessionToken(value);
        return new CredentialMaterial
        {
            Kind = CredentialKind.SessionToken,
            SessionToken = value,
        };
    }

    internal static CredentialMaterial ParseSessionResponse(string json, DateTimeOffset now) =>
        ParseJson(json, now, requireSessionJson: true);

    internal static IReadOnlyList<string> CookieCandidates(CredentialMaterial material)
    {
        if (!string.IsNullOrWhiteSpace(material.CookieHeader))
        {
            return [material.CookieHeader];
        }

        if (string.IsNullOrWhiteSpace(material.SessionToken))
        {
            return [];
        }

        return SupportedCookieNames
            .Select(name => $"{name}={material.SessionToken}")
            .ToArray();
    }

    private static CredentialMaterial ParseJson(string json, DateTimeOffset now, bool requireSessionJson = false)
    {
        JsonDocument document;
        try
        {
            document = JsonDocument.Parse(json, new JsonDocumentOptions
            {
                AllowTrailingCommas = false,
                CommentHandling = JsonCommentHandling.Disallow,
                MaxDepth = 64,
            });
        }
        catch (JsonException ex)
        {
            throw new LensException($"JSON 在第 {ex.LineNumber.GetValueOrDefault() + 1} 行附近不完整或格式错误。", ex);
        }

        using (document)
        {
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object)
            {
                throw new LensException("凭证 JSON 顶层必须是对象。");
            }

            var sessionAccessToken = JsonAccess.String(root, "accessToken");
            if (!string.IsNullOrWhiteSpace(sessionAccessToken))
            {
                var email = JsonAccess.String(root, "user", "email");
                var expires = JsonAccess.DateTime(root, "expires");
                if (string.IsNullOrWhiteSpace(email))
                {
                    throw new LensException("Session JSON 缺少 user.email；请复制 /api/auth/session 的完整内容。");
                }

                if (expires is null)
                {
                    throw new LensException("Session JSON 缺少或无法解析 expires；请复制完整内容。");
                }

                if (expires <= now.AddMinutes(-2))
                {
                    throw new LensException($"Session 已于 {expires.Value.LocalDateTime:yyyy-MM-dd HH:mm:ss} 过期。");
                }

                var accountId = FirstNonEmpty(
                    JsonAccess.String(root, "account", "id"),
                    JsonAccess.String(root, "account", "account_id"));
                return FromAccessToken(
                    sessionAccessToken,
                    CredentialKind.SessionJson,
                    email,
                    accountId,
                    now,
                    expires);
            }

            var codexAccessToken = JsonAccess.String(root, "tokens", "access_token");
            if (!string.IsNullOrWhiteSpace(codexAccessToken) && !requireSessionJson)
            {
                var accountId = JsonAccess.String(root, "tokens", "account_id");
                return FromAccessToken(
                    codexAccessToken,
                    CredentialKind.CodexAuthJson,
                    null,
                    accountId,
                    now);
            }

            if (requireSessionJson)
            {
                throw new LensException("chatgpt.com 返回的会话不完整：没有 accessToken。Session 可能已失效。");
            }

            throw new LensException("JSON 中既没有完整 Session 的 accessToken，也没有 Codex auth.json 的 tokens.access_token。");
        }
    }

    private static CredentialMaterial FromAccessToken(
        string token,
        CredentialKind kind,
        string? email,
        string? accountId,
        DateTimeOffset now,
        DateTimeOffset? sessionExpiresAt = null)
    {
        var jwt = JwtInspector.Inspect(token, now);
        return new CredentialMaterial
        {
            Kind = kind,
            AccessToken = token,
            Email = FirstNonEmpty(email, jwt.Email),
            AccountId = FirstNonEmpty(accountId, jwt.AccountId),
            ExpiresAt = sessionExpiresAt is { } sessionExpiry && sessionExpiry < jwt.ExpiresAt
                ? sessionExpiry
                : jwt.ExpiresAt,
        };
    }

    private static string Normalize(string? input)
    {
        if (string.IsNullOrWhiteSpace(input))
        {
            throw new LensException("请粘贴 Session JSON、Session Token、Access Token 或 Codex auth.json。");
        }

        if (input.Length > MaxCredentialCharacters)
        {
            throw new LensException("输入超过 512 KiB，不像有效的会话凭证。");
        }

        var value = input.Trim().TrimStart('\uFEFF');
        foreach (var character in value)
        {
            if (char.IsControl(character) && character is not '\r' and not '\n' and not '\t')
            {
                throw new LensException("输入含有不可见控制字符，请重新复制凭证。");
            }
        }

        return value;
    }

    private static bool TryExtractCookieHeader(string input, out string cookieHeader, out string sessionToken)
    {
        cookieHeader = string.Empty;
        sessionToken = string.Empty;
        var value = input.StartsWith("Cookie:", StringComparison.OrdinalIgnoreCase)
            ? input[7..].Trim()
            : input;

        if (value.Contains('\r') || value.Contains('\n'))
        {
            return false;
        }

        foreach (var part in value.Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            var separator = part.IndexOf('=');
            if (separator <= 0)
            {
                continue;
            }

            var name = part[..separator].Trim();
            if (!SupportedCookieNames.Contains(name, StringComparer.OrdinalIgnoreCase))
            {
                continue;
            }

            var token = part[(separator + 1)..].Trim();
            if (token.Length == 0)
            {
                throw new LensException($"Cookie {name} 没有值，凭证不完整。");
            }

            // Forward only the recognized session cookie, never unrelated browser cookies.
            cookieHeader = $"{name}={token}";
            sessionToken = token;
            return true;
        }

        return false;
    }

    private static void ValidateOpaqueSessionToken(string value)
    {
        if (value.Length < MinOpaqueSessionLength)
        {
            throw new LensException($"Session Token 只有 {value.Length} 个字符，明显不完整。");
        }

        if (value.Length > MaxOpaqueSessionLength)
        {
            throw new LensException("Session Token 长度异常，可能粘贴了无关内容。");
        }

        if (!OpaqueTokenRegex().IsMatch(value))
        {
            throw new LensException("Session Token 含有空格、换行或非法字符，请只复制 Token 本身。");
        }
    }

    private static bool LooksLikeJwt(string value) =>
        value.Count(character => character == '.') == 2 && value.StartsWith("eyJ", StringComparison.Ordinal);

    private static string? FirstNonEmpty(params string?[] values) =>
        values.FirstOrDefault(value => !string.IsNullOrWhiteSpace(value));

    [GeneratedRegex("^[A-Za-z0-9._~%+\\-=/]+$", RegexOptions.CultureInvariant)]
    private static partial Regex OpaqueTokenRegex();
}
