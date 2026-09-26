using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Net;
using System.Security.Cryptography;
using SubscriptionLens.Core;

namespace SubscriptionLens.Tests;

internal static class Program
{
    private static int _passed;
    private static int _failed;

    public static int Main()
    {
        Run("empty credential is rejected", EmptyCredentialIsRejected);
        Run("short session is rejected", ShortSessionIsRejected);
        Run("valid access token is accepted", ValidAccessTokenIsAccepted);
        Run("Bearer prefix is accepted", BearerPrefixIsAccepted);
        Run("expired access token is rejected", ExpiredAccessTokenIsRejected);
        Run("future access token is rejected", FutureAccessTokenIsRejected);
        Run("truncated JWT signature is rejected", TruncatedJwtSignatureIsRejected);
        Run("session JSON requires email", SessionJsonRequiresEmail);
        Run("complete session JSON is accepted", CompleteSessionJsonIsAccepted);
        Run("Codex auth JSON is accepted", CodexAuthJsonIsAccepted);
        Run("recognized cookie header is accepted", CookieHeaderIsAccepted);
        Run("cookie header drops unrelated cookies", CookieHeaderDropsUnrelatedCookies);
        Run("malformed JSON is rejected", MalformedJsonIsRejected);
        Run("random credential input never escapes validation", RandomInputNeverEscapesValidation);
        Run("out of range JWT timestamps are rejected", OutOfRangeJwtTimestampIsRejected);
        Run("array JSON paths work", ArrayJsonPathsWork);
        Run("preferred account is selected", PreferredAccountIsSelected);
        Run("paid account beats inactive fallback", PaidAccountBeatsInactiveFallback);
        Run("Pro maps to Pro 20X visual", ProMapsToPro20X);
        Run("Pro Lite maps to Pro 5X visual", ProLiteMapsToPro5X);
        Run("unknown product name is not misclassified as Pro", UnknownProductIsNotPro);
        Run("iOS origin maps to App Store", IosOriginMapsToAppStore);
        Run("Google origin maps to Google Play", GoogleOriginMapsToGooglePlay);
        Run("card exposes only returned digits", CardUsesOnlyReturnedDigits);
        Run("card does not invent first six", CardDoesNotInventFirstSix);
        Run("root default payment method is selected", RootDefaultPaymentMethodIsSelected);
        Run("wrapped payment method is parsed", WrappedPaymentMethodIsParsed);
        Run("invoice minor units are normalized", InvoiceMinorUnitsAreNormalized);
        Run("zero-decimal invoice currency is normalized", ZeroDecimalCurrencyIsNormalized);
        Run("three-decimal invoice currency is normalized", ThreeDecimalCurrencyIsNormalized);
        Run("usage windows are parsed", UsageWindowsAreParsed);
        Run("feature quotas remain explicitly unknown", FeatureQuotasRemainUnknown);
        Run("account identifiers are masked", AccountIdentifiersAreMasked);
        Run("partial endpoint failure keeps useful result", PartialFailureKeepsUsefulResult);
        Run("query flow uses GET requests on chatgpt.com only", QueryFlowUsesGetRequestsOnly);
        Run("incomplete exchanged session stops before backend", IncompleteExchangeStopsBeforeBackend);
        Run("HTTP 200 error envelope is not treated as success", ErrorEnvelopeIsNotSuccess);
        Run("deactivated account payload is not treated as success", DeactivatedAccountIsNotSuccess);
        Run("oversized optional response stays partial", OversizedOptionalResponseStaysPartial);
        Run("caller cancellation is preserved", CallerCancellationIsPreserved);
        Run("layout fits a normal work area", LayoutFitsNormalWorkArea);
        Run("layout scales down for high DPI work area", LayoutScalesForHighDpi);
        Run("invalid work area gets safe defaults", InvalidWorkAreaGetsDefaults);

        Console.WriteLine($"Subscription Lens tests: {_passed} passed, {_failed} failed.");
        return _failed == 0 ? 0 : 1;
    }

    private static void EmptyCredentialIsRejected() => False(CredentialParser.Validate(" ").IsValid);

    private static void ShortSessionIsRejected() => False(CredentialParser.Validate("too-short").IsValid);

    private static void ValidAccessTokenIsAccepted()
    {
        var validation = CredentialParser.Validate(Token(DateTimeOffset.UtcNow.AddHours(1)));
        True(validation.IsValid);
        Equal(CredentialKind.AccessToken, validation.Kind);
    }

    private static void BearerPrefixIsAccepted()
    {
        var validation = CredentialParser.Validate($"Bearer {Token(DateTimeOffset.UtcNow.AddHours(1))}");
        True(validation.IsValid);
        Equal(CredentialKind.AccessToken, validation.Kind);
    }

    private static void ExpiredAccessTokenIsRejected()
    {
        var validation = CredentialParser.Validate(Token(DateTimeOffset.UtcNow.AddHours(-1)));
        False(validation.IsValid);
        Contains("过期", validation.Detail);
    }

    private static void FutureAccessTokenIsRejected()
    {
        var validation = CredentialParser.Validate(Token(
            DateTimeOffset.UtcNow.AddHours(2),
            DateTimeOffset.UtcNow.AddMinutes(20)));
        False(validation.IsValid);
        Contains("尚未生效", validation.Detail);
    }

    private static void TruncatedJwtSignatureIsRejected()
    {
        var token = Token(DateTimeOffset.UtcNow.AddHours(1));
        var parts = token.Split('.');
        var validation = CredentialParser.Validate($"{parts[0]}.{parts[1]}.short");
        False(validation.IsValid);
        Contains("签名段", validation.Detail);
    }

    private static void SessionJsonRequiresEmail()
    {
        var json = JsonSerializer.Serialize(new
        {
            accessToken = Token(DateTimeOffset.UtcNow.AddHours(1)),
            expires = DateTimeOffset.UtcNow.AddHours(1),
            user = new { name = "Example" },
        });
        var validation = CredentialParser.Validate(json);
        False(validation.IsValid);
        Contains("user.email", validation.Detail);
    }

    private static void CompleteSessionJsonIsAccepted()
    {
        var json = CompleteSessionJson();
        var validation = CredentialParser.Validate(json);
        True(validation.IsValid);
        Equal(CredentialKind.SessionJson, validation.Kind);
    }

    private static void CodexAuthJsonIsAccepted()
    {
        var json = JsonSerializer.Serialize(new
        {
            auth_mode = "chatgpt",
            tokens = new
            {
                access_token = Token(DateTimeOffset.UtcNow.AddHours(1)),
                id_token = Token(DateTimeOffset.UtcNow.AddHours(1)),
                refresh_token = "fixture",
                account_id = "acct_1234567890",
            },
        });
        var validation = CredentialParser.Validate(json);
        True(validation.IsValid);
        Equal(CredentialKind.CodexAuthJson, validation.Kind);
    }

    private static void CookieHeaderIsAccepted()
    {
        var token = new string('a', 80);
        var validation = CredentialParser.Validate($"Cookie: __Secure-authjs.session-token={token}; theme=dark");
        True(validation.IsValid);
        Equal(CredentialKind.CookieHeader, validation.Kind);
        True(validation.RequiresRemoteCheck);
    }

    private static void CookieHeaderDropsUnrelatedCookies()
    {
        var token = new string('b', 80);
        using var parsed = CredentialParser.Parse($"other=secret; __Secure-next-auth.session-token={token}; theme=dark", DateTimeOffset.UtcNow);
        Equal($"__Secure-next-auth.session-token={token}", parsed.CookieHeader);
    }

    private static void MalformedJsonIsRejected()
    {
        var validation = CredentialParser.Validate("{\"accessToken\":");
        False(validation.IsValid);
        Contains("JSON", validation.Detail);
    }

    private static void RandomInputNeverEscapesValidation()
    {
        for (var sample = 0; sample < 500; sample++)
        {
            var length = RandomNumberGenerator.GetInt32(1, 400);
            var characters = new char[length];
            for (var index = 0; index < length; index++)
            {
                characters[index] = (char)RandomNumberGenerator.GetInt32(0, 128);
            }

            _ = CredentialParser.Validate(new string(characters));
        }
    }

    private static void OutOfRangeJwtTimestampIsRejected()
    {
        var header = Base64Url(JsonSerializer.SerializeToUtf8Bytes(new { alg = "RS256" }));
        var payload = Base64Url(JsonSerializer.SerializeToUtf8Bytes(new { sub = "fixture", exp = long.MaxValue }));
        var validation = CredentialParser.Validate($"{header}.{payload}.{new string('s', 64)}");
        False(validation.IsValid);
        Contains("到期时间", validation.Detail);
    }

    private static void ArrayJsonPathsWork()
    {
        using var json = JsonDocument.Parse("{\"lines\":{\"data\":[{\"description\":\"ChatGPT Plus\"}]}}");
        Equal("ChatGPT Plus", JsonAccess.String(json.RootElement, "lines", "data", "0", "description"));
    }

    private static void PreferredAccountIsSelected()
    {
        using var json = JsonDocument.Parse("""
            {"accounts":{
              "default":{"account":{"account_id":"acct_default"},"entitlement":{"has_active_subscription":false}},
              "acct_paid":{"account":{"account_id":"acct_paid"},"entitlement":{"has_active_subscription":true}}
            }}
            """);
        Equal("acct_paid", SubscriptionNormalizer.ResolveAccountId(json, "acct_paid"));
    }

    private static void PaidAccountBeatsInactiveFallback()
    {
        using var json = JsonDocument.Parse("""
            {"accounts":{
              "acct_off":{"account":{"account_id":"acct_off","is_deactivated":true}},
              "acct_paid":{"account":{"account_id":"acct_paid"},"entitlement":{"has_active_subscription":true}}
            }}
            """);
        Equal("acct_paid", SubscriptionNormalizer.ResolveAccountId(json, null));
    }

    private static void ProMapsToPro20X()
    {
        var result = Normalize(accountsJson: Accounts("pro", "chatgptpro", "chatgpt_web"));
        Equal("Pro 20X", result.Subscription.PlanName);
        Equal(PlanVisual.Pro20X, result.Subscription.Visual);
    }

    private static void ProLiteMapsToPro5X()
    {
        var result = Normalize(accountsJson: Accounts("pro_lite", "chatgptprolite", "chatgpt_web"));
        Equal("Pro 5X", result.Subscription.PlanName);
        Equal(PlanVisual.Pro5X, result.Subscription.Visual);
    }

    private static void UnknownProductIsNotPro()
    {
        var result = Normalize(accountsJson: Accounts("professional_trial", "professional_trial", "chatgpt_web"));
        Equal(PlanVisual.Other, result.Subscription.Visual);
    }

    private static void IosOriginMapsToAppStore()
    {
        var result = Normalize(accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_ios"));
        Equal(PaymentKind.AppleAppStore, result.Payment.Kind);
    }

    private static void GoogleOriginMapsToGooglePlay()
    {
        var result = Normalize(accountsJson: Accounts("plus", "chatgptplusplan", "google_play"));
        Equal(PaymentKind.GooglePlay, result.Payment.Kind);
    }

    private static void CardUsesOnlyReturnedDigits()
    {
        var result = Normalize(
            accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"),
            paymentJson: """{"payment_methods":[{"is_default":true,"card":{"brand":"visa","first6":"424242","last4":"4242","exp_month":12,"exp_year":2030}}]}""");
        Equal("VISA", result.Payment.Brand);
        Equal("424242", result.Payment.First6);
        Equal("4242", result.Payment.Last4);
    }

    private static void CardDoesNotInventFirstSix()
    {
        var result = Normalize(
            accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"),
            paymentJson: """{"payment_methods":[{"card":{"brand":"mastercard","last4":"9876"}}]}""");
        Equal("Mastercard", result.Payment.Brand);
        Null(result.Payment.First6);
        Equal("9876", result.Payment.Last4);
        True(result.Warnings.Any(warning => warning.Contains("前 6 位", StringComparison.Ordinal)));
    }

    private static void RootDefaultPaymentMethodIsSelected()
    {
        var result = Normalize(
            accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"),
            paymentJson: """{"default_payment_method_id":"pm_second","payment_methods":[{"id":"pm_first","card":{"brand":"visa","last4":"1111"}},{"id":"pm_second","card":{"brand":"mastercard","last4":"2222"}}]}""");
        Equal("Mastercard", result.Payment.Brand);
        Equal("2222", result.Payment.Last4);
    }

    private static void WrappedPaymentMethodIsParsed()
    {
        var result = Normalize(
            accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"),
            paymentJson: """{"payment_methods":[{"payment_method":{"id":"pm_wrapped","type":"card","card":{"brand":"visa","last4":"9000"}}}]}""");
        Equal("VISA", result.Payment.Brand);
        Equal("9000", result.Payment.Last4);
    }

    private static void InvoiceMinorUnitsAreNormalized()
    {
        var result = Normalize(
            accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"),
            invoicesJson: """{"data":[{"id":"in_1","amount_paid":1999,"currency":"usd","status":"paid","created":1893456000,"lines":{"data":[{"description":"ChatGPT Plus"}]}}]}""");
        Equal(19.99m, result.BillingRecords.Single().Amount);
        Equal("USD", result.BillingRecords.Single().Currency);
        Equal("ChatGPT Plus", result.BillingRecords.Single().Product);
    }

    private static void ZeroDecimalCurrencyIsNormalized()
    {
        var result = Normalize(
            accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"),
            invoicesJson: """{"data":[{"id":"in_jpy","amount_paid":2000,"currency":"jpy","status":"paid"}]}""");
        Equal(2000m, result.BillingRecords.Single().Amount);
        Equal("JPY 2000", CurrencyRules.Format(result.BillingRecords.Single().Amount!.Value, "JPY"));
    }

    private static void ThreeDecimalCurrencyIsNormalized()
    {
        var result = Normalize(
            accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"),
            invoicesJson: """{"data":[{"id":"in_kwd","amount_paid":1234,"currency":"kwd","status":"paid"}]}""");
        Equal(1.234m, result.BillingRecords.Single().Amount);
        Equal("KWD 1.234", CurrencyRules.Format(result.BillingRecords.Single().Amount!.Value, "KWD"));
    }

    private static void UsageWindowsAreParsed()
    {
        var result = Normalize(
            accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"),
            usageJson: """{"plan_type":"plus","rate_limit":{"allowed":true,"primary_window":{"used_percent":25,"limit_window_seconds":18000,"reset_at":1893456000},"secondary_window":{"used_percent":80,"window_minutes":10080,"reset_at":1894060800}}}""");
        Equal(2, result.CodexWindows.Count);
        Equal(25d, result.CodexWindows[0].UsedPercent);
        Equal(300L, result.CodexWindows[0].WindowMinutes);
    }

    private static void FeatureQuotasRemainUnknown()
    {
        var result = Normalize(accountsJson: Accounts("pro", "chatgptpro", "chatgpt_web"));
        Equal(4, result.FeatureQuotas.Count);
        False(result.FeatureQuotas.Single(quota => quota.Name == "生图").Verified);
        Contains("未返回", result.FeatureQuotas.Single(quota => quota.Name == "Deep Research").DisplayValue);
    }

    private static void AccountIdentifiersAreMasked()
    {
        var result = Normalize(accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"));
        Equal("acct_1…7890", result.AccountIdMasked);
    }

    private static void PartialFailureKeepsUsefulResult()
    {
        var result = Normalize(
            accountsJson: Accounts("plus", "chatgptplusplan", "chatgpt_web"),
            paymentState: ProbeState.Failed);
        Equal("Plus", result.Subscription.PlanName);
        True(result.Warnings.Any(warning => warning.Contains("可选数据", StringComparison.Ordinal)));
    }

    private static void QueryFlowUsesGetRequestsOnly()
    {
        var seen = new List<(HttpMethod Method, Uri Uri, bool Authorized)>();
        using var handler = new FixtureHttpHandler(request =>
        {
            seen.Add((request.Method, request.RequestUri!, request.Headers.Authorization is not null));
            var path = request.RequestUri!.AbsolutePath;
            return path switch
            {
                "/api/auth/session" => JsonResponse(CompleteSessionJson()),
                "/backend-api/accounts/check/v4-2023-04-27" => JsonResponse(Accounts("plus", "chatgptplusplan", "chatgpt_web")),
                "/backend-api/subscriptions" => JsonResponse("""{"id":"sub_fixture","plan_type":"plus","active_until":"2031-01-01T00:00:00Z","will_renew":true}"""),
                "/backend-api/wham/usage" => JsonResponse("""{"rate_limit":{"allowed":true,"primary_window":{"used_percent":10,"window_minutes":300}}}"""),
                "/backend-api/invoices" => JsonResponse("""{"data":[]}"""),
                "/backend-api/payments/payment_methods" => JsonResponse("""{"payment_methods":[]}"""),
                _ => new HttpResponseMessage(HttpStatusCode.NotFound),
            };
        });
        using var client = new HttpClient(handler);
        using var service = new ChatGptQueryService(client);
        var result = service.QueryAsync(new string('s', 96)).GetAwaiter().GetResult();
        Equal("Plus", result.Subscription.PlanName);
        Equal(6, seen.Count);
        True(seen.All(call => call.Method == HttpMethod.Get));
        True(seen.All(call => call.Uri.Scheme == "https" && call.Uri.Host == "chatgpt.com" && call.Uri.Port == 443));
        True(seen.Where(call => call.Uri.AbsolutePath.StartsWith("/backend-api/", StringComparison.Ordinal)).All(call => call.Authorized));
    }

    private static void IncompleteExchangeStopsBeforeBackend()
    {
        var requestCount = 0;
        using var handler = new FixtureHttpHandler(request =>
        {
            requestCount++;
            return JsonResponse("{}");
        });
        using var client = new HttpClient(handler);
        using var service = new ChatGptQueryService(client);
        try
        {
            _ = service.QueryAsync(new string('x', 96)).GetAwaiter().GetResult();
            throw new InvalidOperationException("Expected session exchange to fail.");
        }
        catch (LensException ex)
        {
            Contains("Session", ex.Message);
        }

        Equal(3, requestCount);
    }

    private static void ErrorEnvelopeIsNotSuccess()
    {
        using var handler = new FixtureHttpHandler(request => request.RequestUri!.AbsolutePath switch
        {
            "/backend-api/accounts/check/v4-2023-04-27" => JsonResponse(Accounts("plus", "chatgptplusplan", "chatgpt_web")),
            "/backend-api/subscriptions" => JsonResponse("""{"detail":"account id is required"}"""),
            "/backend-api/wham/usage" => JsonResponse("""{"plan_type":"plus","rate_limit":{"allowed":true}}"""),
            "/backend-api/invoices" => JsonResponse("""{"data":[]}"""),
            "/backend-api/payments/payment_methods" => JsonResponse("""{"payment_methods":[]}"""),
            _ => new HttpResponseMessage(HttpStatusCode.NotFound),
        });
        using var client = new HttpClient(handler, disposeHandler: false);
        using var service = new ChatGptQueryService(client);
        var result = service.QueryAsync(CompleteSessionJson()).GetAwaiter().GetResult();
        Equal(ProbeState.Unavailable, result.Probes.Single(probe => probe.Name == "当前订阅").State);
        True(result.Warnings.Count > 0);
    }

    private static void DeactivatedAccountIsNotSuccess()
    {
        using var handler = new FixtureHttpHandler(request => request.RequestUri!.AbsolutePath switch
        {
            "/backend-api/accounts/check/v4-2023-04-27" => JsonResponse("""{"accounts":{"acct_1234567890":{"account":{"account_id":"acct_1234567890","is_deactivated":true}}}}"""),
            "/backend-api/subscriptions" => JsonResponse("""{"id":"sub_fixture","plan_type":"plus"}"""),
            "/backend-api/wham/usage" => JsonResponse("""{"plan_type":"plus","rate_limit":{"allowed":true}}"""),
            "/backend-api/invoices" => JsonResponse("""{"data":[]}"""),
            "/backend-api/payments/payment_methods" => JsonResponse("""{"payment_methods":[]}"""),
            _ => new HttpResponseMessage(HttpStatusCode.NotFound),
        });
        using var client = new HttpClient(handler, disposeHandler: false);
        using var service = new ChatGptQueryService(client);
        var result = service.QueryAsync(CompleteSessionJson()).GetAwaiter().GetResult();
        Equal(ProbeState.Unavailable, result.Probes.Single(probe => probe.Name == "账户状态").State);
        Equal("Plus", result.Subscription.PlanName);
    }

    private static void OversizedOptionalResponseStaysPartial()
    {
        using var handler = new FixtureHttpHandler(request => request.RequestUri!.AbsolutePath switch
        {
            "/backend-api/accounts/check/v4-2023-04-27" => JsonResponse(Accounts("plus", "chatgptplusplan", "chatgpt_web")),
            "/backend-api/subscriptions" => JsonResponse("""{"id":"sub_fixture","plan_type":"plus"}"""),
            "/backend-api/wham/usage" => JsonResponse("""{"plan_type":"plus","rate_limit":{"allowed":true}}"""),
            "/backend-api/invoices" => JsonResponse("""{"data":[]}"""),
            "/backend-api/payments/payment_methods" => JsonResponse($"{{\"padding\":\"{new string('x', (2 * 1024 * 1024) + 1)}\"}}"),
            _ => new HttpResponseMessage(HttpStatusCode.NotFound),
        });
        using var client = new HttpClient(handler, disposeHandler: false);
        using var service = new ChatGptQueryService(client);
        var result = service.QueryAsync(CompleteSessionJson()).GetAwaiter().GetResult();
        var paymentProbe = result.Probes.Single(probe => probe.Name == "支付方式");
        Equal(ProbeState.Failed, paymentProbe.State);
        Contains("安全上限", paymentProbe.Message);
        Equal("Plus", result.Subscription.PlanName);
    }

    private static void CallerCancellationIsPreserved()
    {
        using var handler = new FixtureHttpHandler(_ => JsonResponse("{}"));
        using var client = new HttpClient(handler, disposeHandler: false);
        using var service = new ChatGptQueryService(client);
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        try
        {
            _ = service.QueryAsync(CompleteSessionJson(), cancellationToken: cancellation.Token)
                .GetAwaiter()
                .GetResult();
            throw new InvalidOperationException("Expected cancellation.");
        }
        catch (OperationCanceledException)
        {
            // Expected: cancellation must not be converted into a credential or endpoint error.
        }
    }

    private static void LayoutFitsNormalWorkArea()
    {
        var plan = LayoutPlanner.Calculate(1920, 1040);
        Equal(1160d, plan.Width);
        Equal(720d, plan.Height);
        Equal(1d, plan.EstimatedScale);
    }

    private static void LayoutScalesForHighDpi()
    {
        var plan = LayoutPlanner.Calculate(1280, 680);
        Equal(1160d, plan.Width);
        Equal(632d, plan.Height);
        True(plan.EstimatedScale < 1d);
        True(plan.Height < 680d);
    }

    private static void InvalidWorkAreaGetsDefaults()
    {
        var plan = LayoutPlanner.Calculate(double.NaN, -1);
        Equal(1160d, plan.Width);
        Equal(720d, plan.Height);
        Equal(1d, plan.EstimatedScale);
    }

    private static InspectionResult Normalize(
        string? accountsJson = null,
        string? subscriptionJson = null,
        string? usageJson = null,
        string? invoicesJson = null,
        string? paymentJson = null,
        ProbeState paymentState = ProbeState.Success)
    {
        using var credential = CredentialParser.Parse(CompleteSessionJson(), DateTimeOffset.UtcNow);
        using var accounts = Outcome("账户状态", accountsJson, accountsJson is null ? ProbeState.Skipped : ProbeState.Success);
        using var subscription = Outcome("当前订阅", subscriptionJson, subscriptionJson is null ? ProbeState.Skipped : ProbeState.Success);
        using var usage = Outcome("Codex 额度", usageJson, usageJson is null ? ProbeState.Skipped : ProbeState.Success);
        using var invoices = Outcome("网页账单", invoicesJson, invoicesJson is null ? ProbeState.Skipped : ProbeState.Success);
        using var payment = Outcome("支付方式", paymentJson, paymentJson is null ? paymentState : ProbeState.Success);
        return SubscriptionNormalizer.Normalize(
            credential,
            accounts,
            subscription,
            usage,
            invoices,
            payment,
            DateTimeOffset.Parse("2030-01-01T00:00:00Z", CultureInfo.InvariantCulture));
    }

    private static FetchOutcome Outcome(string name, string? json, ProbeState state) =>
        new(name, state, json is null ? null : JsonDocument.Parse(json), state == ProbeState.Success ? "读取成功" : "fixture unavailable");

    private static string Accounts(string planType, string subscriptionPlan, string origin) =>
        JsonSerializer.Serialize(new
        {
            accounts = new Dictionary<string, object>
            {
                ["acct_1234567890"] = new
                {
                    account = new { account_id = "acct_1234567890", plan_type = planType },
                    entitlement = new
                    {
                        has_active_subscription = true,
                        subscription_plan = subscriptionPlan,
                        expires_at = "2031-01-01T00:00:00Z",
                        billing_currency = "USD",
                    },
                    last_active_subscription = new { purchase_origin_platform = origin, will_renew = true },
                },
            },
        });

    private static string CompleteSessionJson() => JsonSerializer.Serialize(new
    {
        accessToken = Token(DateTimeOffset.UtcNow.AddHours(2)),
        expires = DateTimeOffset.UtcNow.AddHours(2),
        user = new { email = "owner@example.com" },
        account = new { id = "acct_1234567890" },
    });

    private static string Token(DateTimeOffset expires, DateTimeOffset? notBefore = null)
    {
        var header = Base64Url(JsonSerializer.SerializeToUtf8Bytes(new { alg = "RS256", typ = "JWT" }));
        var claims = new Dictionary<string, object>
        {
            ["sub"] = "user-fixture",
            ["exp"] = expires.ToUnixTimeSeconds(),
            ["email"] = "owner@example.com",
            ["https://api.openai.com/auth"] = new Dictionary<string, string>
            {
                ["chatgpt_account_id"] = "acct_1234567890",
            },
        };
        if (notBefore is { } value)
        {
            claims["nbf"] = value.ToUnixTimeSeconds();
        }

        var payload = Base64Url(JsonSerializer.SerializeToUtf8Bytes(claims));
        return $"{header}.{payload}.{new string('s', 64)}";
    }

    private static string Base64Url(byte[] data) =>
        Convert.ToBase64String(data).TrimEnd('=').Replace('+', '-').Replace('/', '_');

    private static HttpResponseMessage JsonResponse(string json) => new(HttpStatusCode.OK)
    {
        Content = new StringContent(json, Encoding.UTF8, "application/json"),
    };

    [SuppressMessage(
        "Design",
        "CA1031:Do not catch general exception types",
        Justification = "The test harness must record every failed test and continue through the remaining independent cases.")]
    private static void Run(string name, Action action)
    {
        try
        {
            action();
            _passed++;
            Console.WriteLine($"PASS  {name}");
        }
        catch (Exception ex)
        {
            _failed++;
            Console.WriteLine($"FAIL  {name}: {ex.Message}");
        }
    }

    private static void True(bool value)
    {
        if (!value) throw new InvalidOperationException("Expected true.");
    }

    private static void False(bool value)
    {
        if (value) throw new InvalidOperationException("Expected false.");
    }

    private static void Null(object? value)
    {
        if (value is not null) throw new InvalidOperationException($"Expected null, got {value}.");
    }

    private static void Equal<T>(T expected, T actual)
    {
        if (!EqualityComparer<T>.Default.Equals(expected, actual))
        {
            throw new InvalidOperationException($"Expected {expected}, got {actual}.");
        }
    }

    private static void Contains(string expected, string actual)
    {
        if (!actual.Contains(expected, StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"Expected '{actual}' to contain '{expected}'.");
        }
    }

    private sealed class FixtureHttpHandler(Func<HttpRequestMessage, HttpResponseMessage> responder) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken) =>
            cancellationToken.IsCancellationRequested
                ? Task.FromCanceled<HttpResponseMessage>(cancellationToken)
                : Task.FromResult(responder(request));
    }
}
