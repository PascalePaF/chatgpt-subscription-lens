using System.IO.Compression;
using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;

namespace SubscriptionLens.Core;

public sealed class ChatGptQueryService : IDisposable
{
    private static readonly Uri SessionUri = new("https://chatgpt.com/api/auth/session");
    private static readonly Uri AccountCheckBaseUri = new("https://chatgpt.com/backend-api/accounts/check/v4-2023-04-27");
    private static readonly Uri UsageUri = new("https://chatgpt.com/backend-api/wham/usage");
    private const int DefaultMaxResponseBytes = 2 * 1024 * 1024;
    private const int InvoiceMaxResponseBytes = 8 * 1024 * 1024;
    private readonly HttpClient _client;
    private readonly HttpClientHandler? _ownedHandler;
    private readonly bool _ownsClient;

    public ChatGptQueryService()
    {
        _ownedHandler = new HttpClientHandler
        {
            AllowAutoRedirect = false,
            AutomaticDecompression = DecompressionMethods.GZip | DecompressionMethods.Deflate | DecompressionMethods.Brotli,
            CheckCertificateRevocationList = true,
            UseCookies = false,
            MaxConnectionsPerServer = 6,
        };
        _client = new HttpClient(_ownedHandler, disposeHandler: false)
        {
            Timeout = Timeout.InfiniteTimeSpan,
        };
        _ownsClient = true;
    }

    internal ChatGptQueryService(HttpClient client)
    {
        _client = client;
        _ownsClient = false;
    }

    public async Task<InspectionResult> QueryAsync(
        string credentialInput,
        IProgress<string>? progress = null,
        CancellationToken cancellationToken = default)
    {
        progress?.Report("正在检查凭证完整性…");
        using var credential = CredentialParser.Parse(credentialInput, DateTimeOffset.UtcNow);

        if (credential.NeedsSessionExchange)
        {
            progress?.Report("正在向 ChatGPT 核验 Session…");
            await ExchangeSessionAsync(credential, cancellationToken).ConfigureAwait(false);
        }

        if (string.IsNullOrWhiteSpace(credential.AccessToken))
        {
            throw new LensException("完整会话中没有 Access Token，查询已停止。");
        }

        progress?.Report("正在读取账户与订阅状态…");
        var accountUri = AccountCheckUri();
        using var accounts = await FetchJsonAsync(
            "账户状态",
            accountUri,
            credential.AccessToken,
            credential.AccountId,
            DefaultMaxResponseBytes,
            cancellationToken).ConfigureAwait(false);

        var resolvedAccountId = SubscriptionNormalizer.ResolveAccountId(accounts.Document, credential.AccountId);
        if (!string.IsNullOrWhiteSpace(resolvedAccountId))
        {
            credential.AccountId = resolvedAccountId;
        }

        progress?.Report("正在并行读取订阅、额度与账单…");
        var subscriptionTask = FetchAccountScopedAsync(
            "当前订阅",
            "/backend-api/subscriptions",
            credential,
            DefaultMaxResponseBytes,
            cancellationToken);
        var invoiceTask = FetchAccountScopedAsync(
            "网页账单",
            "/backend-api/invoices",
            credential,
            InvoiceMaxResponseBytes,
            cancellationToken,
            ("limit", "12"));
        var paymentTask = FetchAccountScopedAsync(
            "支付方式",
            "/backend-api/payments/payment_methods",
            credential,
            DefaultMaxResponseBytes,
            cancellationToken);
        var usageTask = FetchJsonAsync(
            "Codex 额度",
            UsageUri,
            credential.AccessToken,
            credential.AccountId,
            DefaultMaxResponseBytes,
            cancellationToken);

        var parallelTasks = new[] { subscriptionTask, invoiceTask, paymentTask, usageTask };
        try
        {
            await Task.WhenAll(parallelTasks).ConfigureAwait(false);
        }
        catch
        {
            foreach (var completedTask in parallelTasks.Where(task => task.IsCompletedSuccessfully))
            {
                var completed = await completedTask.ConfigureAwait(false);
                completed.Dispose();
            }

            throw;
        }

        using var subscription = await subscriptionTask.ConfigureAwait(false);
        using var invoices = await invoiceTask.ConfigureAwait(false);
        using var payment = await paymentTask.ConfigureAwait(false);
        using var usage = await usageTask.ConfigureAwait(false);

        var usefulSuccesses = new[] { accounts, subscription, usage }
            .Count(outcome => outcome.State == ProbeState.Success);
        if (usefulSuccesses == 0)
        {
            var reason = string.Join("；", new[] { accounts, subscription, usage }
                .Where(outcome => outcome.State is ProbeState.Failed or ProbeState.Unavailable)
                .Select(outcome => $"{outcome.Name}：{outcome.Message}"));
            throw new LensException($"凭证通过了本地检查，但没有任何核心只读接口可用。{reason}");
        }

        progress?.Report("正在整理结果…");
        return SubscriptionNormalizer.Normalize(
            credential,
            accounts,
            subscription,
            usage,
            invoices,
            payment,
            DateTimeOffset.Now);
    }

    private async Task ExchangeSessionAsync(CredentialMaterial credential, CancellationToken cancellationToken)
    {
        var candidates = CredentialParser.CookieCandidates(credential);
        if (candidates.Count == 0)
        {
            throw new LensException("Session Token 不完整，无法构造会话核验请求。");
        }

        var failures = new List<string>();
        foreach (var cookie in candidates)
        {
            try
            {
                await ExchangeSessionCandidateAsync(credential, cookie, cancellationToken).ConfigureAwait(false);
                return;
            }
            catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
            {
                failures.Add("会话核验超过 20 秒");
            }
            catch (LensException ex)
            {
                failures.Add(ex.Message);
            }
            catch (HttpRequestException)
            {
                failures.Add("网络连接失败");
            }
        }

        throw new LensException($"Session 未通过 chatgpt.com 核验。{failures.FirstOrDefault() ?? "会话可能已过期。"}");
    }

    private async Task ExchangeSessionCandidateAsync(
        CredentialMaterial credential,
        string cookie,
        CancellationToken cancellationToken)
    {
        using var request = CreateRequest(SessionUri, accessToken: null, accountId: null);
        request.Headers.TryAddWithoutValidation("Cookie", cookie);
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromSeconds(20));
        using var response = await _client
            .SendAsync(request, HttpCompletionOption.ResponseHeadersRead, timeout.Token)
            .ConfigureAwait(false);
        if (response.StatusCode != HttpStatusCode.OK)
        {
            throw new LensException(StatusMessage(response.StatusCode));
        }

        var json = await ReadLimitedStringAsync(response, DefaultMaxResponseBytes, timeout.Token)
            .ConfigureAwait(false);
        using var complete = CredentialParser.ParseSessionResponse(json, DateTimeOffset.UtcNow);
        credential.AccessToken = complete.AccessToken;
        credential.Email = complete.Email;
        credential.AccountId = complete.AccountId;
        credential.ExpiresAt = complete.ExpiresAt;
    }

    private Task<FetchOutcome> FetchAccountScopedAsync(
        string name,
        string path,
        CredentialMaterial credential,
        int maxBytes,
        CancellationToken cancellationToken,
        params (string Name, string Value)[] extraQuery)
    {
        if (string.IsNullOrWhiteSpace(credential.AccountId))
        {
            return Task.FromResult(new FetchOutcome(
                name,
                ProbeState.Skipped,
                null,
                "凭证和账户响应均未提供 account_id"));
        }

        var query = new List<(string Name, string Value)>(extraQuery)
        {
            ("account_id", credential.AccountId),
        };
        var uri = BuildChatGptUri(path, query);
        return FetchJsonAsync(
            name,
            uri,
            credential.AccessToken!,
            credential.AccountId,
            maxBytes,
            cancellationToken);
    }

    private async Task<FetchOutcome> FetchJsonAsync(
        string name,
        Uri uri,
        string accessToken,
        string? accountId,
        int maxBytes,
        CancellationToken cancellationToken)
    {
        EnsureAllowedUri(uri);
        using var request = CreateRequest(uri, accessToken, accountId);

        try
        {
            using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            timeout.CancelAfter(TimeSpan.FromSeconds(20));
            using var response = await _client
                .SendAsync(request, HttpCompletionOption.ResponseHeadersRead, timeout.Token)
                .ConfigureAwait(false);
            if (response.StatusCode != HttpStatusCode.OK)
            {
                var state = response.StatusCode is HttpStatusCode.BadRequest or HttpStatusCode.Forbidden or HttpStatusCode.NotFound
                    ? ProbeState.Unavailable
                    : ProbeState.Failed;
                return new FetchOutcome(name, state, null, StatusMessage(response.StatusCode));
            }

            var contentType = response.Content.Headers.ContentType?.MediaType;
            if (contentType is not null && !contentType.Contains("json", StringComparison.OrdinalIgnoreCase))
            {
                return new FetchOutcome(name, ProbeState.Unavailable, null, "服务返回了非 JSON 内容");
            }

            var json = await ReadLimitedBytesAsync(response, maxBytes, timeout.Token).ConfigureAwait(false);
            JsonDocument document;
            try
            {
                document = JsonDocument.Parse(json, new JsonDocumentOptions
                {
                    AllowTrailingCommas = false,
                    CommentHandling = JsonCommentHandling.Disallow,
                    MaxDepth = 96,
                });
            }
            catch (JsonException)
            {
                return new FetchOutcome(name, ProbeState.Failed, null, "服务响应不是有效 JSON");
            }

            var semanticProblem = PayloadProblem(name, document.RootElement);
            if (semanticProblem is not null)
            {
                document.Dispose();
                return new FetchOutcome(name, ProbeState.Unavailable, null, semanticProblem);
            }

            return new FetchOutcome(name, ProbeState.Success, document, "读取成功");
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return new FetchOutcome(name, ProbeState.Failed, null, "请求超过 20 秒");
        }
        catch (HttpRequestException)
        {
            return new FetchOutcome(name, ProbeState.Failed, null, "网络连接失败");
        }
        catch (LensException ex)
        {
            return new FetchOutcome(name, ProbeState.Failed, null, ex.Message);
        }
    }

    private static HttpRequestMessage CreateRequest(Uri uri, string? accessToken, string? accountId)
    {
        EnsureAllowedUri(uri);
        var request = new HttpRequestMessage(HttpMethod.Get, uri);
        request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
        var version = typeof(ChatGptQueryService).Assembly.GetName().Version;
        request.Headers.UserAgent.ParseAdd($"SubscriptionLens/{version?.Major}.{version?.Minor}.{version?.Build} (Windows; local-read-only)");
        request.Headers.TryAddWithoutValidation("OAI-Language", "zh-CN");
        request.Headers.Referrer = new Uri("https://chatgpt.com/");
        if (!string.IsNullOrWhiteSpace(accessToken))
        {
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", accessToken);
        }

        if (!string.IsNullOrWhiteSpace(accountId))
        {
            request.Headers.TryAddWithoutValidation("ChatGPT-Account-ID", accountId);
        }

        return request;
    }

    private static async Task<string> ReadLimitedStringAsync(
        HttpResponseMessage response,
        int maxBytes,
        CancellationToken cancellationToken)
    {
        var bytes = await ReadLimitedBytesAsync(response, maxBytes, cancellationToken).ConfigureAwait(false);
        return Encoding.UTF8.GetString(bytes);
    }

    private static async Task<byte[]> ReadLimitedBytesAsync(
        HttpResponseMessage response,
        int maxBytes,
        CancellationToken cancellationToken)
    {
        if (response.Content.Headers.ContentLength is > 0 and var length && length > maxBytes)
        {
            throw new LensException("服务响应超过本地安全上限");
        }

        using var stream = await response.Content.ReadAsStreamAsync(cancellationToken).ConfigureAwait(false);
        using var buffer = new MemoryStream(Math.Min(maxBytes, 64 * 1024));
        var chunk = new byte[16 * 1024];
        while (true)
        {
            var read = await stream.ReadAsync(chunk.AsMemory(), cancellationToken).ConfigureAwait(false);
            if (read == 0)
            {
                break;
            }

            if (buffer.Length + read > maxBytes)
            {
                throw new LensException("服务响应超过本地安全上限");
            }

            await buffer.WriteAsync(chunk.AsMemory(0, read), cancellationToken).ConfigureAwait(false);
        }

        return buffer.ToArray();
    }

    private static Uri AccountCheckUri()
    {
        var javascriptOffset = -(int)DateTimeOffset.Now.Offset.TotalMinutes;
        return new Uri($"{AccountCheckBaseUri}?timezone_offset_min={javascriptOffset}");
    }

    private static string? PayloadProblem(string name, JsonElement root)
    {
        if (root.ValueKind == JsonValueKind.Object)
        {
            var detail = JsonAccess.At(root, "detail");
            var error = JsonAccess.At(root, "error");
            if (detail is { ValueKind: not JsonValueKind.Null and not JsonValueKind.Undefined } ||
                error is { ValueKind: not JsonValueKind.Null and not JsonValueKind.Undefined })
            {
                return "服务返回了错误信封，没有可用数据";
            }
        }

        return name switch
        {
            "账户状态" when !HasNonEmptyObject(root, "accounts") => "账户响应缺少 accounts 数据",
            "账户状态" when !HasUsableAccount(root) => "账户响应中的工作区均已停用",
            "当前订阅" when !HasAny(root, "id", "plan_type", "active_until", "billing_period") =>
                "订阅响应缺少可识别字段",
            "Codex 额度" when !HasAny(root, "plan_type", "rate_limit", "additional_rate_limits", "credits") =>
                "额度响应缺少可识别字段",
            "网页账单" when root.ValueKind != JsonValueKind.Array &&
                         !HasArray(root, "data", "invoices", "items", "transactions") =>
                "账单响应缺少列表字段",
            "支付方式" when root.ValueKind != JsonValueKind.Array &&
                         !HasArray(root, "payment_methods", "data", "items", "cards") =>
                "支付方式响应缺少列表字段",
            _ => null,
        };
    }

    private static bool HasAny(JsonElement root, params string[] names) =>
        names.Any(name => JsonAccess.At(root, name) is { ValueKind: not JsonValueKind.Null and not JsonValueKind.Undefined });

    private static bool HasArray(JsonElement root, params string[] names) =>
        names.Any(name => JsonAccess.At(root, name) is { ValueKind: JsonValueKind.Array });

    private static bool HasNonEmptyObject(JsonElement root, string name)
    {
        var value = JsonAccess.At(root, name);
        return value is { ValueKind: JsonValueKind.Object } && value.Value.EnumerateObject().Any();
    }

    private static bool HasUsableAccount(JsonElement root)
    {
        var accounts = JsonAccess.At(root, "accounts");
        return accounts is { ValueKind: JsonValueKind.Object } && accounts.Value
            .EnumerateObject()
            .Any(property =>
                JsonAccess.Bool(property.Value, "account", "is_deactivated") != true &&
                JsonAccess.Bool(property.Value, "is_deactivated") != true);
    }

    private static Uri BuildChatGptUri(string path, IEnumerable<(string Name, string Value)> query)
    {
        var encoded = string.Join("&", query.Select(pair =>
            $"{Uri.EscapeDataString(pair.Name)}={Uri.EscapeDataString(pair.Value)}"));
        return new UriBuilder(Uri.UriSchemeHttps, "chatgpt.com")
        {
            Path = path,
            Query = encoded,
        }.Uri;
    }

    private static void EnsureAllowedUri(Uri uri)
    {
        if (!uri.Scheme.Equals(Uri.UriSchemeHttps, StringComparison.OrdinalIgnoreCase) ||
            !uri.Host.Equals("chatgpt.com", StringComparison.OrdinalIgnoreCase) ||
            uri.Port != 443)
        {
            throw new LensException("安全策略拒绝了非 chatgpt.com 的请求地址。");
        }
    }

    private static string StatusMessage(HttpStatusCode statusCode) => statusCode switch
    {
        HttpStatusCode.BadRequest => "接口不适用于此账户或请求字段已变化（HTTP 400）",
        HttpStatusCode.Unauthorized => "凭证已过期或未获授权（HTTP 401）",
        HttpStatusCode.PaymentRequired => "账户付款需要处理，或工作区已停用（HTTP 402）",
        HttpStatusCode.Forbidden => "账户或网络被拒绝访问（HTTP 403）",
        HttpStatusCode.NotFound => "此账户没有该项数据，或接口已变化（HTTP 404）",
        HttpStatusCode.TooManyRequests => "请求过于频繁，请稍后再试（HTTP 429）",
        _ => $"服务返回 HTTP {(int)statusCode}",
    };

    public void Dispose()
    {
        if (_ownsClient)
        {
            _client.Dispose();
            _ownedHandler?.Dispose();
        }
    }
}
