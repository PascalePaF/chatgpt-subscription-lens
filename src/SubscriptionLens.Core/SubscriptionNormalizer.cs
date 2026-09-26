using System.Globalization;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace SubscriptionLens.Core;

internal static partial class SubscriptionNormalizer
{
    public static string? ResolveAccountId(JsonDocument? accountsDocument, string? preferredAccountId)
    {
        var selected = SelectAccount(accountsDocument, preferredAccountId);
        return selected is null
            ? preferredAccountId
            : FirstNonEmpty(
                JsonAccess.String(selected.Value.Entry, "account", "account_id"),
                selected.Value.Key.Equals("default", StringComparison.OrdinalIgnoreCase) ? null : selected.Value.Key,
                preferredAccountId);
    }

    public static InspectionResult Normalize(
        CredentialMaterial credential,
        FetchOutcome accounts,
        FetchOutcome subscription,
        FetchOutcome usage,
        FetchOutcome invoices,
        FetchOutcome payment,
        DateTimeOffset now)
    {
        var selected = SelectAccount(accounts.Document, credential.AccountId);
        var accountEntry = selected?.Entry;
        var portal = subscription.Document?.RootElement;

        var summary = NormalizeSubscription(portal, accountEntry, now);
        var paymentSummary = NormalizePayment(payment.Document, summary.PurchaseOrigin);
        var billingRecords = NormalizeInvoices(invoices.Document);
        var codexWindows = NormalizeUsage(usage.Document);

        var email = FirstNonEmpty(
            credential.Email,
            accountEntry is { } entry ? JsonAccess.String(entry, "account", "email") : null,
            "邮箱未返回")!;
        var accountId = FirstNonEmpty(
            ResolveAccountId(accounts.Document, credential.AccountId),
            credential.AccountId);

        var probes = new[] { accounts, subscription, usage, invoices, payment }
            .Select(outcome => new ProbeStatus(outcome.Name, outcome.State, outcome.Message))
            .ToArray();
        var warnings = BuildWarnings(summary, paymentSummary, probes);

        var featureQuotas = new List<FeatureQuota>
        {
            new()
            {
                Name = "Chat 对话",
                DisplayValue = "未返回实时余量",
                Detail = "ChatGPT 没有独立只读剩余额度接口",
            },
            new()
            {
                Name = "网页端 Pro",
                DisplayValue = summary.Visual is PlanVisual.Pro20X or PlanVisual.Pro5X ? "套餐已确认" : "不适用 / 未确认",
                Detail = "仅确认订阅，不把动态限流当作固定额度",
                Verified = summary.Visual is PlanVisual.Pro20X or PlanVisual.Pro5X,
            },
            new()
            {
                Name = "生图",
                DisplayValue = "未返回实时余量",
                Detail = "余量只会随真实会话事件出现，本工具不发起会话",
            },
            new()
            {
                Name = "Deep Research",
                DisplayValue = "未返回实时余量",
                Detail = "余量只会随真实会话事件出现，本工具不消耗次数",
            },
        };

        return new InspectionResult
        {
            Email = email,
            AccountIdMasked = MaskAccountId(accountId),
            CheckedAt = now,
            Subscription = summary,
            Payment = paymentSummary,
            CodexWindows = codexWindows,
            FeatureQuotas = featureQuotas,
            BillingRecords = billingRecords,
            Probes = probes,
            Warnings = warnings,
        };
    }

    private static SubscriptionSummary NormalizeSubscription(
        JsonElement? portal,
        JsonElement? accountEntry,
        DateTimeOffset now)
    {
        var account = accountEntry is { } entry ? JsonAccess.At(entry, "account") : null;
        var entitlement = accountEntry is { } entEntry ? JsonAccess.At(entEntry, "entitlement") : null;
        var last = accountEntry is { } lastEntry ? JsonAccess.At(lastEntry, "last_active_subscription") : null;

        var rawPlan = FirstNonEmpty(
            portal is { } p ? JsonAccess.String(p, "plan_type") : null,
            account is { } a ? JsonAccess.String(a, "plan_type") : null,
            entitlement is { } e ? JsonAccess.String(e, "subscription_plan") : null,
            "free")!;
        var (planName, visual) = NormalizePlan(rawPlan);

        var startsAt = FirstDate(
            portal is { } p1 ? JsonAccess.DateTime(p1, "active_start") : null,
            portal is { } p2 ? JsonAccess.DateTime(p2, "current_period_start") : null);
        var expiresAt = FirstDate(
            portal is { } p3 ? JsonAccess.DateTime(p3, "active_until") : null,
            entitlement is { } e1 ? JsonAccess.DateTime(e1, "expires_at") : null,
            entitlement is { } e2 ? JsonAccess.DateTime(e2, "cancels_at") : null,
            entitlement is { } e3 ? JsonAccess.DateTime(e3, "renews_at") : null);

        var active = FirstBool(
            entitlement is { } e4 ? JsonAccess.Bool(e4, "has_active_subscription") : null,
            portal is { } p4 ? JsonAccess.Bool(p4, "active") : null);
        if (active is null && visual == PlanVisual.Free)
        {
            active = false;
        }
        else if (active is null && expiresAt is { } expiry)
        {
            active = expiry > now;
        }

        return new SubscriptionSummary
        {
            PlanName = planName,
            Visual = visual,
            IsActive = active,
            StartsAt = startsAt,
            ExpiresAt = expiresAt,
            WillRenew = FirstBool(
                portal is { } p5 ? JsonAccess.Bool(p5, "will_renew") : null,
                last is { } l1 ? JsonAccess.Bool(l1, "will_renew") : null),
            BillingPeriod = FirstNonEmpty(
                portal is { } p6 ? JsonAccess.String(p6, "billing_period") : null,
                entitlement is { } e5 ? JsonAccess.String(e5, "billing_period") : null),
            Currency = FirstNonEmpty(
                portal is { } p7 ? JsonAccess.String(p7, "billing_currency") : null,
                entitlement is { } e6 ? JsonAccess.String(e6, "billing_currency") : null)?.ToUpperInvariant(),
            PurchaseOrigin = FirstNonEmpty(
                last is { } l2 ? JsonAccess.String(l2, "purchase_origin_platform") : null,
                portal is { } p8 ? JsonAccess.String(p8, "purchase_origin_platform") : null,
                portal is { } p9 ? JsonAccess.String(p9, "purchase_origin") : null),
            IsDelinquent = FirstBool(
                portal is { } p10 ? JsonAccess.Bool(p10, "is_delinquent") : null,
                entitlement is { } e7 ? JsonAccess.Bool(e7, "is_delinquent") : null),
            CancellationOutcome = FirstNonEmpty(
                portal is { } p11 ? JsonAccess.String(p11, "cancellation_outcome") : null,
                last is { } l3 ? JsonAccess.String(l3, "cancellation_outcome") : null),
        };
    }

    private static PaymentSummary NormalizePayment(JsonDocument? paymentDocument, string? purchaseOrigin)
    {
        var origin = purchaseOrigin?.ToUpperInvariant() ?? string.Empty;
        if (origin.Contains("IOS", StringComparison.Ordinal) || origin.Contains("APPLE", StringComparison.Ordinal))
        {
            return new PaymentSummary { Kind = PaymentKind.AppleAppStore, Label = "Apple App Store" };
        }

        if (origin.Contains("ANDROID", StringComparison.Ordinal) ||
            origin.Contains("GOOGLE", StringComparison.Ordinal) ||
            origin.Contains("PLAY", StringComparison.Ordinal))
        {
            return new PaymentSummary { Kind = PaymentKind.GooglePlay, Label = "Google Play" };
        }

        if (paymentDocument is null)
        {
            return new PaymentSummary();
        }

        var root = paymentDocument.RootElement;
        var methods = FirstArray(root, "payment_methods", "data", "items", "cards");
        if (methods is null)
        {
            return new PaymentSummary();
        }

        var defaultId = FirstNonEmpty(
            JsonAccess.String(root, "default_payment_method_id"),
            JsonAccess.String(root, "default_payment_method"));
        JsonElement? selected = null;
        foreach (var method in methods.Value.EnumerateArray())
        {
            if (method.ValueKind != JsonValueKind.Object)
            {
                continue;
            }

            var candidate = JsonAccess.At(method, "payment_method") ?? method;
            selected ??= candidate;
            var candidateId = FirstNonEmpty(JsonAccess.String(candidate, "id"), JsonAccess.String(method, "id"));
            if ((!string.IsNullOrWhiteSpace(defaultId) && candidateId == defaultId) ||
                JsonAccess.Bool(candidate, "is_default") == true ||
                JsonAccess.Bool(method, "is_default") == true ||
                JsonAccess.Bool(candidate, "default") == true ||
                JsonAccess.Bool(method, "default") == true)
            {
                selected = candidate;
                break;
            }
        }

        if (selected is null)
        {
            return new PaymentSummary();
        }

        var value = selected.Value;
        var cardElement = JsonAccess.At(value, "card");
        var card = cardElement ?? value;
        var paymentType = JsonAccess.String(value, "type");
        var hasCard = cardElement is not null ||
                      paymentType?.Equals("card", StringComparison.OrdinalIgnoreCase) == true ||
                      JsonAccess.String(card, "last4") is not null;
        if (!hasCard)
        {
            return new PaymentSummary
            {
                Kind = PaymentKind.Unknown,
                Label = string.IsNullOrWhiteSpace(paymentType)
                    ? "未返回支付方式"
                    : CultureInfo.InvariantCulture.TextInfo.ToTitleCase(paymentType.Replace('_', ' ')),
            };
        }

        var brand = FirstNonEmpty(JsonAccess.String(card, "brand"), JsonAccess.String(value, "brand"));
        var first6 = DigitsOrNull(FirstNonEmpty(
            JsonAccess.String(card, "first6"),
            JsonAccess.String(card, "bin"),
            JsonAccess.String(card, "iin")), 6);
        var last4 = DigitsOrNull(FirstNonEmpty(
            JsonAccess.String(card, "last4"),
            JsonAccess.String(value, "last4")), 4);
        var normalizedBrand = NormalizeCardBrand(brand);

        return new PaymentSummary
        {
            Kind = PaymentKind.Card,
            Label = string.IsNullOrWhiteSpace(normalizedBrand) ? "银行卡" : normalizedBrand,
            Brand = normalizedBrand,
            First6 = first6,
            Last4 = last4,
            ExpMonth = ToInt(JsonAccess.Int64(card, "exp_month")),
            ExpYear = ToInt(JsonAccess.Int64(card, "exp_year")),
            IsDefault = (!string.IsNullOrWhiteSpace(defaultId) && JsonAccess.String(value, "id") == defaultId) ||
                        JsonAccess.Bool(value, "is_default") == true ||
                        JsonAccess.Bool(value, "default") == true,
        };
    }

    private static BillingRecord[] NormalizeInvoices(JsonDocument? invoiceDocument)
    {
        if (invoiceDocument is null)
        {
            return [];
        }

        var root = invoiceDocument.RootElement;
        var entries = FirstArray(root, "data", "invoices", "items", "transactions");
        if (entries is null)
        {
            return [];
        }

        var results = new List<BillingRecord>();
        var index = 0;
        foreach (var item in entries.Value.EnumerateArray())
        {
            if (item.ValueKind != JsonValueKind.Object)
            {
                continue;
            }

            var currency = JsonAccess.String(item, "currency")?.ToUpperInvariant();
            var amount = FirstDecimal(
                JsonAccess.Decimal(item, "amount_paid"),
                JsonAccess.Decimal(item, "total"),
                JsonAccess.Decimal(item, "amount_due"));
            if (amount is { } minorAmount)
            {
                amount = CurrencyRules.FromMinorUnits(minorAmount, currency);
            }
            else
            {
                amount = FirstDecimal(JsonAccess.Decimal(item, "amount"), JsonAccess.Decimal(item, "price"));
            }

            var product = FirstNonEmpty(
                JsonAccess.String(item, "description"),
                JsonAccess.String(item, "lines", "data", "0", "description"),
                JsonAccess.String(item, "product"),
                "ChatGPT 订阅")!;

            results.Add(new BillingRecord
            {
                Id = FirstNonEmpty(
                    JsonAccess.String(item, "id"),
                    JsonAccess.String(item, "invoice_id"),
                    $"invoice-{index}")!,
                CreatedAt = FirstDate(
                    JsonAccess.DateTime(item, "created"),
                    JsonAccess.DateTime(item, "created_at"),
                    JsonAccess.DateTime(item, "paid_at"),
                    JsonAccess.DateTime(item, "period_start")),
                Amount = amount,
                Currency = currency,
                Status = FirstNonEmpty(
                    JsonAccess.String(item, "status"),
                    JsonAccess.Bool(item, "paid") == true ? "paid" : null,
                    "未知")!,
                Product = product,
                InvoiceUrl = SafeHttpsUrl(FirstNonEmpty(
                    JsonAccess.String(item, "invoice_pdf"),
                    JsonAccess.String(item, "hosted_invoice_url"))),
            });
            index++;
        }

        return results
            .OrderByDescending(record => record.CreatedAt ?? DateTimeOffset.MinValue)
            .Take(12)
            .ToArray();
    }

    private static QuotaWindow[] NormalizeUsage(JsonDocument? usageDocument)
    {
        if (usageDocument is null)
        {
            return [];
        }

        var root = usageDocument.RootElement;
        var windows = new List<QuotaWindow>();
        var rateLimit = JsonAccess.At(root, "rate_limit");
        if (rateLimit is { } limit)
        {
            AddWindow(windows, "codex-primary", "Codex · 短周期", JsonAccess.At(limit, "primary_window"), JsonAccess.Bool(limit, "allowed"));
            AddWindow(windows, "codex-secondary", "Codex · 长周期", JsonAccess.At(limit, "secondary_window"), JsonAccess.Bool(limit, "allowed"));
        }

        var additional = JsonAccess.At(root, "additional_rate_limits");
        if (additional is { ValueKind: JsonValueKind.Array })
        {
            var index = 0;
            foreach (var entry in additional.Value.EnumerateArray())
            {
                var details = JsonAccess.At(entry, "details") ?? entry;
                var id = FirstNonEmpty(
                    JsonAccess.String(entry, "metered_limit_name"),
                    JsonAccess.String(entry, "limit_name"),
                    $"additional-{index}")!;
                var label = FirstNonEmpty(
                    JsonAccess.String(entry, "limit_name"),
                    JsonAccess.String(details, "limit_name"),
                    $"Codex · {id}")!;
                AddWindow(windows, $"{id}-primary", label, JsonAccess.At(details, "primary_window"), JsonAccess.Bool(details, "allowed"));
                index++;
            }
        }

        return windows.Take(6).ToArray();
    }

    private static void AddWindow(
        List<QuotaWindow> windows,
        string id,
        string name,
        JsonElement? element,
        bool? allowed)
    {
        if (element is null || element.Value.ValueKind != JsonValueKind.Object)
        {
            return;
        }

        var used = JsonAccess.Double(element.Value, "used_percent");
        var minutes = FirstLong(
            JsonAccess.Int64(element.Value, "window_minutes"),
            SecondsToMinutes(JsonAccess.Int64(element.Value, "limit_window_seconds")));
        var reset = FirstDate(
            JsonAccess.DateTime(element.Value, "reset_at"),
            ResetAfter(element.Value));
        if (used is null && minutes is null && reset is null)
        {
            return;
        }

        windows.Add(new QuotaWindow
        {
            Id = id,
            Name = name,
            UsedPercent = used is { } value ? Math.Clamp(value, 0d, 100d) : null,
            WindowMinutes = minutes,
            ResetsAt = reset,
            Allowed = allowed,
        });
    }

    private static (string Name, PlanVisual Visual) NormalizePlan(string raw)
    {
        var compact = NonAlphaNumericRegex().Replace(raw.ToUpperInvariant(), string.Empty);
        if (compact.Contains("PROLITE", StringComparison.Ordinal) ||
            compact.Contains("PRO5X", StringComparison.Ordinal) ||
            compact.Contains("PRO5", StringComparison.Ordinal))
        {
            return ("Pro 5X", PlanVisual.Pro5X);
        }

        if (compact is "PRO" or "CHATGPTPRO" or "CHATGPTPROPLAN" or "PRO20" or "PRO20X" ||
            compact.StartsWith("CHATGPTPRO20", StringComparison.Ordinal))
        {
            return ("Pro 20X", PlanVisual.Pro20X);
        }

        if (compact.Contains("PLUS", StringComparison.Ordinal))
        {
            return ("Plus", PlanVisual.Plus);
        }

        if (compact.Contains("FREE", StringComparison.Ordinal) || compact.Contains("NOTPURCHASED", StringComparison.Ordinal))
        {
            return ("Free", PlanVisual.Free);
        }

        return (CultureInfo.InvariantCulture.TextInfo.ToTitleCase(raw.Replace('_', ' ')), PlanVisual.Other);
    }

    private static List<string> BuildWarnings(
        SubscriptionSummary summary,
        PaymentSummary payment,
        IReadOnlyList<ProbeStatus> probes)
    {
        var warnings = new List<string>();
        if (summary.IsDelinquent == true)
        {
            warnings.Add("账户被标记为欠款/逾期，请在原购买平台核对。");
        }

        if (summary.WillRenew == false && summary.IsActive == true)
        {
            warnings.Add("当前订阅有效，但不会在本期结束时自动续费。");
        }

        if (payment.Kind == PaymentKind.Card && payment.Last4 is not null && payment.First6 is null)
        {
            warnings.Add("支付接口没有返回卡号前 6 位；为避免猜测，仅显示尾号 4 位。");
        }

        if (payment.Kind is PaymentKind.AppleAppStore or PaymentKind.GooglePlay)
        {
            warnings.Add("ChatGPT 只返回当前购买来源；完整商店历史需在对应 Apple/Google 账户中查看。");
        }

        var failed = probes.Count(probe => probe.State is ProbeState.Failed or ProbeState.Unavailable);
        if (failed > 0)
        {
            warnings.Add($"{failed} 项可选数据未读取；已成功的数据仍然有效。");
        }

        return warnings;
    }

    private static (string Key, JsonElement Entry)? SelectAccount(JsonDocument? document, string? preferredAccountId)
    {
        if (document is null)
        {
            return null;
        }

        var accounts = JsonAccess.At(document.RootElement, "accounts");
        if (accounts is not { ValueKind: JsonValueKind.Object })
        {
            return null;
        }

        var candidates = accounts.Value.EnumerateObject().ToArray();
        if (!string.IsNullOrWhiteSpace(preferredAccountId))
        {
            var exact = candidates
                .Where(property => property.Name.Equals(preferredAccountId, StringComparison.OrdinalIgnoreCase))
                .Select(property => (JsonProperty?)property)
                .FirstOrDefault();
            if (exact is { } exactProperty)
            {
                return (exactProperty.Name, exactProperty.Value);
            }
        }

        var defaultAccount = candidates
            .Where(property => property.Name.Equals("default", StringComparison.OrdinalIgnoreCase))
            .Select(property => (JsonProperty?)property)
            .FirstOrDefault();
        if (defaultAccount is { } defaultProperty && !IsDeactivated(defaultProperty.Value))
        {
            return (defaultProperty.Name, defaultProperty.Value);
        }

        var paid = candidates
            .Where(property =>
                !IsDeactivated(property.Value) && JsonAccess.Bool(property.Value, "entitlement", "has_active_subscription") == true)
            .Select(property => (JsonProperty?)property)
            .FirstOrDefault();
        if (paid is { } paidProperty)
        {
            return (paidProperty.Name, paidProperty.Value);
        }

        var active = candidates
            .Where(property => !IsDeactivated(property.Value))
            .Select(property => (JsonProperty?)property)
            .FirstOrDefault();
        return active is { } activeProperty ? (activeProperty.Name, activeProperty.Value) : null;
    }

    private static bool IsDeactivated(JsonElement account) =>
        JsonAccess.Bool(account, "account", "is_deactivated") == true ||
        JsonAccess.Bool(account, "is_deactivated") == true;

    private static JsonElement? FirstArray(JsonElement root, params string[] names)
    {
        if (root.ValueKind == JsonValueKind.Array)
        {
            return root;
        }

        foreach (var name in names)
        {
            var candidate = JsonAccess.At(root, name);
            if (candidate is { ValueKind: JsonValueKind.Array })
            {
                return candidate;
            }
        }

        return null;
    }

    private static DateTimeOffset? ResetAfter(JsonElement element)
    {
        var seconds = JsonAccess.Int64(element, "reset_after_seconds");
        return seconds is >= 0 and <= 366 * 24 * 3600
            ? DateTimeOffset.UtcNow.AddSeconds(seconds.Value)
            : null;
    }

    private static long? SecondsToMinutes(long? seconds) =>
        seconds is null ? null : Math.Max(1, (long)Math.Ceiling(seconds.Value / 60d));

    private static string MaskAccountId(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return "账户未返回";
        }

        if (value.Length <= 10)
        {
            return "••••";
        }

        return $"{value[..6]}…{value[^4..]}";
    }

    private static string? NormalizeCardBrand(string? brand)
    {
        if (string.IsNullOrWhiteSpace(brand))
        {
            return null;
        }

        return brand.ToUpperInvariant() switch
        {
            "VISA" => "VISA",
            "MASTERCARD" or "MASTER_CARD" or "MASTER CARD" => "Mastercard",
            "AMEX" or "AMERICAN_EXPRESS" => "American Express",
            _ => CultureInfo.InvariantCulture.TextInfo.ToTitleCase(brand.Replace('_', ' ')),
        };
    }

    private static string? DigitsOrNull(string? value, int expectedLength) =>
        value is not null && value.Length == expectedLength && value.All(char.IsDigit) ? value : null;

    private static int? ToInt(long? value) => value is >= int.MinValue and <= int.MaxValue ? (int)value.Value : null;

    private static Uri? SafeHttpsUrl(string? value) =>
        Uri.TryCreate(value, UriKind.Absolute, out var uri) && uri.Scheme == Uri.UriSchemeHttps
            ? uri
            : null;

    private static T? FirstValue<T>(params T?[] values) where T : struct =>
        values.FirstOrDefault(value => value.HasValue);

    private static DateTimeOffset? FirstDate(params DateTimeOffset?[] values) => FirstValue(values);
    private static bool? FirstBool(params bool?[] values) => FirstValue(values);
    private static decimal? FirstDecimal(params decimal?[] values) => FirstValue(values);
    private static long? FirstLong(params long?[] values) => FirstValue(values);
    private static string? FirstNonEmpty(params string?[] values) =>
        values.FirstOrDefault(value => !string.IsNullOrWhiteSpace(value));

    [GeneratedRegex("[^A-Z0-9]+", RegexOptions.CultureInvariant)]
    private static partial Regex NonAlphaNumericRegex();
}
