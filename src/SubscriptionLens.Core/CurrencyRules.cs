using System.Globalization;

namespace SubscriptionLens.Core;

public static class CurrencyRules
{
    private static readonly HashSet<string> ZeroDecimalCurrencies = new(StringComparer.OrdinalIgnoreCase)
    {
        "BIF", "CLP", "DJF", "GNF", "JPY", "KMF", "KRW", "MGA",
        "PYG", "RWF", "UGX", "VND", "VUV", "XAF", "XOF", "XPF",
    };

    private static readonly HashSet<string> ThreeDecimalCurrencies = new(StringComparer.OrdinalIgnoreCase)
    {
        "BHD", "JOD", "KWD", "OMR", "TND",
    };

    public static int MinorUnitDigits(string? currency)
    {
        if (string.IsNullOrWhiteSpace(currency))
        {
            return 2;
        }

        if (ZeroDecimalCurrencies.Contains(currency))
        {
            return 0;
        }

        return ThreeDecimalCurrencies.Contains(currency) ? 3 : 2;
    }

    public static decimal FromMinorUnits(decimal amount, string? currency) =>
        amount / DecimalPowerOfTen(MinorUnitDigits(currency));

    public static string Format(decimal amount, string? currency)
    {
        var digits = MinorUnitDigits(currency);
        var format = digits == 0 ? "0" : $"0.{new string('0', digits)}";
        var number = amount.ToString(format, CultureInfo.InvariantCulture);
        return string.IsNullOrWhiteSpace(currency) ? number : $"{currency.ToUpperInvariant()} {number}";
    }

    private static decimal DecimalPowerOfTen(int exponent) => exponent switch
    {
        0 => 1m,
        3 => 1000m,
        _ => 100m,
    };
}
