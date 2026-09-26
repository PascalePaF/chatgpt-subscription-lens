using System.Globalization;
using System.Text.Json;

namespace SubscriptionLens.Core;

internal static class JsonAccess
{
    public static JsonElement? At(JsonElement root, params string[] path)
    {
        var current = root;
        foreach (var segment in path)
        {
            if (current.ValueKind == JsonValueKind.Array &&
                int.TryParse(segment, NumberStyles.None, CultureInfo.InvariantCulture, out var index) &&
                index >= 0 && index < current.GetArrayLength())
            {
                current = current[index];
                continue;
            }

            if (current.ValueKind != JsonValueKind.Object ||
                !TryGetPropertyCaseInsensitive(current, segment, out current))
            {
                return null;
            }
        }

        return current;
    }

    public static string? String(JsonElement root, params string[] path)
    {
        var value = At(root, path);
        if (value is null)
        {
            return null;
        }

        return value.Value.ValueKind switch
        {
            JsonValueKind.String => NullIfWhiteSpace(value.Value.GetString()),
            JsonValueKind.Number => value.Value.GetRawText(),
            _ => null,
        };
    }

    public static bool? Bool(JsonElement root, params string[] path)
    {
        var value = At(root, path);
        if (value is null)
        {
            return null;
        }

        return value.Value.ValueKind switch
        {
            JsonValueKind.True => true,
            JsonValueKind.False => false,
            JsonValueKind.String when bool.TryParse(value.Value.GetString(), out var parsed) => parsed,
            _ => null,
        };
    }

    public static long? Int64(JsonElement root, params string[] path)
    {
        var value = At(root, path);
        if (value is null)
        {
            return null;
        }

        if (value.Value.ValueKind == JsonValueKind.Number && value.Value.TryGetInt64(out var numeric))
        {
            return numeric;
        }

        return value.Value.ValueKind == JsonValueKind.String &&
               long.TryParse(value.Value.GetString(), NumberStyles.Integer, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : null;
    }

    public static double? Double(JsonElement root, params string[] path)
    {
        var value = At(root, path);
        if (value is null)
        {
            return null;
        }

        if (value.Value.ValueKind == JsonValueKind.Number && value.Value.TryGetDouble(out var numeric) && double.IsFinite(numeric))
        {
            return numeric;
        }

        return value.Value.ValueKind == JsonValueKind.String &&
               double.TryParse(value.Value.GetString(), NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed) &&
               double.IsFinite(parsed)
            ? parsed
            : null;
    }

    public static decimal? Decimal(JsonElement root, params string[] path)
    {
        var value = At(root, path);
        if (value is null)
        {
            return null;
        }

        if (value.Value.ValueKind == JsonValueKind.Number && value.Value.TryGetDecimal(out var numeric))
        {
            return numeric;
        }

        return value.Value.ValueKind == JsonValueKind.String &&
               decimal.TryParse(value.Value.GetString(), NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed)
            ? parsed
            : null;
    }

    public static DateTimeOffset? DateTime(JsonElement root, params string[] path)
    {
        var value = At(root, path);
        if (value is null)
        {
            return null;
        }

        if (value.Value.ValueKind == JsonValueKind.Number && value.Value.TryGetInt64(out var unix))
        {
            return FromUnix(unix);
        }

        if (value.Value.ValueKind != JsonValueKind.String)
        {
            return null;
        }

        var text = value.Value.GetString();
        if (string.IsNullOrWhiteSpace(text))
        {
            return null;
        }

        if (long.TryParse(text, NumberStyles.Integer, CultureInfo.InvariantCulture, out unix))
        {
            return FromUnix(unix);
        }

        return DateTimeOffset.TryParse(
            text,
            CultureInfo.InvariantCulture,
            DateTimeStyles.AssumeUniversal | DateTimeStyles.AllowWhiteSpaces,
            out var parsed)
            ? parsed
            : null;
    }

    public static bool TryGetPropertyCaseInsensitive(JsonElement root, string name, out JsonElement value)
    {
        if (root.ValueKind == JsonValueKind.Object)
        {
            if (root.TryGetProperty(name, out value))
            {
                return true;
            }

            foreach (var property in root.EnumerateObject())
            {
                if (property.Name.Equals(name, StringComparison.OrdinalIgnoreCase))
                {
                    value = property.Value;
                    return true;
                }
            }
        }

        value = default;
        return false;
    }

    private static DateTimeOffset? FromUnix(long value)
    {
        try
        {
            // Some upstream timestamps are milliseconds despite their name.
            return Math.Abs(value) > 99_999_999_999
                ? DateTimeOffset.FromUnixTimeMilliseconds(value)
                : DateTimeOffset.FromUnixTimeSeconds(value);
        }
        catch (ArgumentOutOfRangeException)
        {
            return null;
        }
    }

    private static string? NullIfWhiteSpace(string? value) =>
        string.IsNullOrWhiteSpace(value) ? null : value;
}
