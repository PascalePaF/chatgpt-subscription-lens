namespace SubscriptionLens.Core;

public sealed record WindowLayoutPlan(
    double Width,
    double Height,
    double MinWidth,
    double MinHeight,
    double EstimatedScale);

public static class LayoutPlanner
{
    public const double DesignWidth = 1160d;
    public const double DesignHeight = 680d;
    private const double PreferredWindowHeight = 720d;
    private const double WorkAreaMargin = 24d;
    private const double NonClientHeightAllowance = 40d;

    public static WindowLayoutPlan Calculate(double workAreaWidth, double workAreaHeight)
    {
        if (!double.IsFinite(workAreaWidth) || !double.IsFinite(workAreaHeight) ||
            workAreaWidth <= 0d || workAreaHeight <= 0d)
        {
            return new WindowLayoutPlan(DesignWidth, PreferredWindowHeight, 900d, 600d, 1d);
        }

        var availableWidth = Math.Max(320d, workAreaWidth - (WorkAreaMargin * 2d));
        var availableHeight = Math.Max(320d, workAreaHeight - (WorkAreaMargin * 2d));
        var width = Math.Min(DesignWidth, availableWidth);
        var height = Math.Min(PreferredWindowHeight, availableHeight);
        var contentHeight = Math.Max(1d, height - NonClientHeightAllowance);
        var scale = Math.Min(1d, Math.Min(width / DesignWidth, contentHeight / DesignHeight));

        return new WindowLayoutPlan(
            width,
            height,
            Math.Min(900d, width),
            Math.Min(560d, height),
            Math.Clamp(scale, 0.1d, 1d));
    }
}
