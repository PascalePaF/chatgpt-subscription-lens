using System.Text.Json;

namespace SubscriptionLens.Core;

public enum CredentialKind
{
    SessionJson,
    CodexAuthJson,
    AccessToken,
    SessionToken,
    CookieHeader,
}

public sealed class CredentialMaterial : IDisposable
{
    public required CredentialKind Kind { get; init; }
    public string? AccessToken { get; set; }
    public string? SessionToken { get; set; }
    public string? CookieHeader { get; set; }
    public string? Email { get; set; }
    public string? AccountId { get; set; }
    public DateTimeOffset? ExpiresAt { get; set; }

    public bool NeedsSessionExchange => string.IsNullOrWhiteSpace(AccessToken);

    public void Dispose()
    {
        AccessToken = null;
        SessionToken = null;
        CookieHeader = null;
        Email = null;
        AccountId = null;
        ExpiresAt = null;
    }
}

public sealed record CredentialValidation(
    bool IsValid,
    CredentialKind? Kind,
    string Title,
    string Detail,
    bool RequiresRemoteCheck);

public enum PlanVisual
{
    Pro20X,
    Pro5X,
    Plus,
    Free,
    Other,
}

public sealed record SubscriptionSummary
{
    public string PlanName { get; init; } = "未确认";
    public PlanVisual Visual { get; init; } = PlanVisual.Other;
    public bool? IsActive { get; init; }
    public DateTimeOffset? StartsAt { get; init; }
    public DateTimeOffset? ExpiresAt { get; init; }
    public bool? WillRenew { get; init; }
    public string? BillingPeriod { get; init; }
    public string? Currency { get; init; }
    public string? PurchaseOrigin { get; init; }
    public bool? IsDelinquent { get; init; }
    public string? CancellationOutcome { get; init; }
}

public enum PaymentKind
{
    Card,
    AppleAppStore,
    GooglePlay,
    Unknown,
}

public sealed record PaymentSummary
{
    public PaymentKind Kind { get; init; } = PaymentKind.Unknown;
    public string Label { get; init; } = "未返回支付方式";
    public string? Brand { get; init; }
    public string? First6 { get; init; }
    public string? Last4 { get; init; }
    public int? ExpMonth { get; init; }
    public int? ExpYear { get; init; }
    public bool IsDefault { get; init; }
}

public sealed record BillingRecord
{
    public string Id { get; init; } = string.Empty;
    public DateTimeOffset? CreatedAt { get; init; }
    public decimal? Amount { get; init; }
    public string? Currency { get; init; }
    public string Status { get; init; } = "未知";
    public string Product { get; init; } = "ChatGPT 订阅";
    public string? InvoiceUrl { get; init; }
}

public sealed record QuotaWindow
{
    public string Id { get; init; } = "codex";
    public string Name { get; init; } = "Codex";
    public double? UsedPercent { get; init; }
    public long? WindowMinutes { get; init; }
    public DateTimeOffset? ResetsAt { get; init; }
    public bool? Allowed { get; init; }
}

public sealed record FeatureQuota
{
    public string Name { get; init; } = string.Empty;
    public string DisplayValue { get; init; } = "未公开";
    public string Detail { get; init; } = "当前只读接口未返回";
    public bool Verified { get; init; }
}

public enum ProbeState
{
    Success,
    Unavailable,
    Failed,
    Skipped,
}

public sealed record ProbeStatus(string Name, ProbeState State, string Message);

public sealed record InspectionResult
{
    public string Email { get; init; } = "邮箱未返回";
    public string AccountIdMasked { get; init; } = "账户未返回";
    public DateTimeOffset CheckedAt { get; init; } = DateTimeOffset.Now;
    public SubscriptionSummary Subscription { get; init; } = new();
    public PaymentSummary Payment { get; init; } = new();
    public IReadOnlyList<QuotaWindow> CodexWindows { get; init; } = [];
    public IReadOnlyList<FeatureQuota> FeatureQuotas { get; init; } = [];
    public IReadOnlyList<BillingRecord> BillingRecords { get; init; } = [];
    public IReadOnlyList<ProbeStatus> Probes { get; init; } = [];
    public IReadOnlyList<string> Warnings { get; init; } = [];

    public int SuccessfulProbeCount => Probes.Count(p => p.State == ProbeState.Success);
    public int AttemptedProbeCount => Probes.Count(p => p.State != ProbeState.Skipped);
}

internal sealed record FetchOutcome(
    string Name,
    ProbeState State,
    JsonDocument? Document,
    string Message) : IDisposable
{
    public void Dispose() => Document?.Dispose();
}

public sealed class LensException : Exception
{
    public LensException(string message) : base(message)
    {
    }

    public LensException(string message, Exception innerException) : base(message, innerException)
    {
    }
}
