using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Reflection;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using SubscriptionLens.Core;

namespace SubscriptionLens.App;

public sealed partial class MainWindow : Window, IDisposable
{
    private readonly ChatGptQueryService _queryService = new();
    private readonly DispatcherTimer _validationTimer;
    private CancellationTokenSource? _queryCancellation;
    private bool _inputIsValid;
    private bool _disposed;

    public MainWindow()
    {
        InitializeComponent();
        var workArea = SystemParameters.WorkArea;
        var layout = LayoutPlanner.Calculate(workArea.Width, workArea.Height);
        Width = layout.Width;
        Height = layout.Height;
        MinWidth = layout.MinWidth;
        MinHeight = layout.MinHeight;
        var version = Assembly.GetExecutingAssembly().GetName().Version;
        VersionText.Text = $"v{version?.Major}.{version?.Minor}.{version?.Build} · 开源本地版";
        _validationTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(220) };
        _validationTimer.Tick += (_, _) =>
        {
            _validationTimer.Stop();
            ValidateCredentialInput();
        };
        Closed += (_, _) => Dispose();
    }

    internal bool ValidateLayoutContract() =>
        Width <= SystemParameters.WorkArea.Width &&
        Height <= SystemParameters.WorkArea.Height &&
        LayoutScaler.Stretch == Stretch.Uniform &&
        LayoutScaler.StretchDirection == StretchDirection.DownOnly &&
        Math.Abs(CredentialBox.Height - 82d) < 0.1 &&
        Math.Abs(CredentialBox.MinHeight - CredentialBox.MaxHeight) < 0.1 &&
        QueryButton.Visibility == Visibility.Visible &&
        ConnectPanel.Visibility == Visibility.Visible;

    private void CredentialBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        CredentialPlaceholder.Visibility = string.IsNullOrEmpty(CredentialBox.Text)
            ? Visibility.Visible
            : Visibility.Collapsed;
        _validationTimer.Stop();
        _validationTimer.Start();
    }

    private void ValidateCredentialInput()
    {
        var text = CredentialBox.Text;
        if (string.IsNullOrWhiteSpace(text))
        {
            _inputIsValid = false;
            ValidationBorder.Background = BrushFrom("#F1EEE8");
            ValidationDot.Fill = BrushFrom("#9B958C");
            ValidationTitle.Text = "等待输入";
            ValidationDetail.Text = "粘贴后会立即检查，不会自动查询。";
            UpdateQueryButton();
            return;
        }

        var validation = CredentialParser.Validate(text);
        _inputIsValid = validation.IsValid;
        ValidationBorder.Background = BrushFrom(validation.IsValid ? "#EAF4ED" : "#F9E9E5");
        ValidationDot.Fill = BrushFrom(validation.IsValid ? "#4A9C6B" : "#C45D48");
        ValidationTitle.Text = validation.Title;
        ValidationDetail.Text = validation.Detail;
        UpdateQueryButton();
    }

    private void ConsentCheckBox_Changed(object sender, RoutedEventArgs e) => UpdateQueryButton();

    private void UpdateQueryButton()
    {
        QueryButton.IsEnabled = _inputIsValid && ConsentCheckBox.IsChecked == true && _queryCancellation is null;
    }

    [SuppressMessage(
        "Design",
        "CA1031:Do not catch general exception types",
        Justification = "A top-level desktop UI event must convert unexpected non-fatal failures into a safe message instead of terminating the process.")]
    private async void QueryButton_Click(object sender, RoutedEventArgs e)
    {
        ValidateCredentialInput();
        if (!_inputIsValid || ConsentCheckBox.IsChecked != true)
        {
            return;
        }

        _queryCancellation?.Dispose();
        var queryCancellation = new CancellationTokenSource();
        _queryCancellation = queryCancellation;
        UpdateQueryButton();
        BusyStatusText.Text = "正在检查凭证完整性…";
        BusyOverlay.Visibility = Visibility.Visible;
        var credential = CredentialBox.Text;
        var progress = new Progress<string>(message => BusyStatusText.Text = message);

        try
        {
            var result = await _queryService
                .QueryAsync(credential, progress, queryCancellation.Token)
                .ConfigureAwait(true);
            RenderResult(result);
            CredentialBox.Clear();
            ConsentCheckBox.IsChecked = false;
            ConnectPanel.Visibility = Visibility.Collapsed;
            ResultPanel.Visibility = Visibility.Visible;
        }
        catch (OperationCanceledException)
        {
            // User-initiated cancellation is already visible through the overlay closing.
        }
        catch (LensException ex)
        {
            MessageBox.Show(this, ex.Message, "查询没有完成", MessageBoxButton.OK, MessageBoxImage.Warning);
        }
        catch (Exception)
        {
            MessageBox.Show(
                this,
                "查询遇到未预期错误。凭证没有写入磁盘；请检查网络后重试。",
                "查询没有完成",
                MessageBoxButton.OK,
                MessageBoxImage.Error);
        }
        finally
        {
            credential = string.Empty;
            BusyOverlay.Visibility = Visibility.Collapsed;
            queryCancellation.Dispose();
            if (ReferenceEquals(_queryCancellation, queryCancellation))
            {
                _queryCancellation = null;
            }
            UpdateQueryButton();
        }
    }

    private void RenderResult(InspectionResult result)
    {
        ResultEmail.Text = result.Email;
        ResultMeta.Text = $"{result.AccountIdMasked}  ·  {result.CheckedAt.ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture)}  ·  {result.SuccessfulProbeCount}/{result.AttemptedProbeCount} 项读取成功";
        AccountMaskedText.Text = result.AccountIdMasked;

        ApplyPlanTheme(result.Subscription.Visual);
        PlanNameText.Text = result.Subscription.PlanName;
        PlanStateText.Text = result.Subscription.IsActive switch
        {
            true => "订阅有效",
            false when result.Subscription.Visual == PlanVisual.Free => "免费账户",
            false => "订阅未激活",
            _ => "状态未确认",
        };
        RemainingText.Text = FormatRemaining(result.Subscription.ExpiresAt);
        ExpiryText.Text = result.Subscription.ExpiresAt?.LocalDateTime.ToString("yyyy-MM-dd HH:mm", CultureInfo.InvariantCulture) ?? "未返回";
        var renewal = result.Subscription.WillRenew switch
        {
            true => "自动续费",
            false => "不续费",
            _ => "续费未知",
        };
        RenewalText.Text = string.IsNullOrWhiteSpace(result.Subscription.Currency)
            ? renewal
            : $"{renewal} · {result.Subscription.Currency}";

        RenderPayment(result.Payment);
        RenderCodexQuotas(result.CodexWindows);
        RenderFeatureQuotas(result.FeatureQuotas);
        RenderBilling(result.BillingRecords);

        var warning = result.Warnings.Count > 0 ? result.Warnings[0] : null;
        WarningBorder.Visibility = warning is null ? Visibility.Collapsed : Visibility.Visible;
        WarningText.Text = warning ?? string.Empty;
    }

    private void ApplyPlanTheme(PlanVisual visual)
    {
        var (background, accent, foreground, muted) = visual switch
        {
            PlanVisual.Pro20X => ("#201E1A", "#D6B86A", "#FFF8E5", "#AAA399"),
            PlanVisual.Pro5X => ("#17385E", "#8EC7FF", "#F4FAFF", "#AFC6DC"),
            PlanVisual.Plus => ("#215C47", "#9DE0BC", "#F2FFF8", "#B2D0C3"),
            PlanVisual.Free => ("#565A5D", "#D8DBDD", "#FFFFFF", "#C4C7C9"),
            _ => ("#3E3A36", "#E0C0AA", "#FFFFFF", "#C6BEB7"),
        };
        PlanHeroBorder.Background = BrushFrom(background);
        PlanStateText.Foreground = BrushFrom(accent);
        PlanNameText.Foreground = BrushFrom(foreground);
        RemainingText.Foreground = BrushFrom(foreground);
        ExpiryText.Foreground = BrushFrom(foreground);
        RenewalText.Foreground = BrushFrom(foreground);

        foreach (var label in FindVisualChildren<TextBlock>(PlanHeroBorder)
                     .Where(block => block.Text is "剩余时间" or "到期时间" or "续费 / 币种"))
        {
            label.Foreground = BrushFrom(muted);
        }
    }

    private void RenderPayment(PaymentSummary payment)
    {
        PaymentLabelText.Text = payment.Label;
        PaymentNumberText.Text = payment.Kind switch
        {
            PaymentKind.Card when payment.First6 is not null && payment.Last4 is not null =>
                $"{payment.First6}  ••••••  {payment.Last4}",
            PaymentKind.Card when payment.Last4 is not null => $"••••  ••••  ••••  {payment.Last4}",
            PaymentKind.AppleAppStore => "由 Apple 管理",
            PaymentKind.GooglePlay => "由 Google 管理",
            _ => "—",
        };
        PaymentMetaText.Text = payment.Kind == PaymentKind.Card && payment.ExpMonth is not null && payment.ExpYear is not null
            ? $"有效期 {payment.ExpMonth:00}/{payment.ExpYear}  ·  仅显示上游返回的脱敏卡号"
            : payment.Kind is PaymentKind.AppleAppStore or PaymentKind.GooglePlay
                ? "完整购买历史请在对应商店账户中查看"
                : "当前只读接口没有返回支付方式";

        GenericPaymentIcon.Visibility = Visibility.Collapsed;
        VisaIcon.Visibility = Visibility.Collapsed;
        MastercardIcon.Visibility = Visibility.Collapsed;
        AppleIcon.Visibility = Visibility.Collapsed;
        GoogleIcon.Visibility = Visibility.Collapsed;
        if (payment.Kind == PaymentKind.AppleAppStore)
        {
            AppleIcon.Visibility = Visibility.Visible;
        }
        else if (payment.Kind == PaymentKind.GooglePlay)
        {
            GoogleIcon.Visibility = Visibility.Visible;
        }
        else if (payment.Brand?.Equals("VISA", StringComparison.OrdinalIgnoreCase) == true)
        {
            VisaIcon.Visibility = Visibility.Visible;
        }
        else if (payment.Brand?.Contains("Mastercard", StringComparison.OrdinalIgnoreCase) == true)
        {
            MastercardIcon.Visibility = Visibility.Visible;
        }
        else
        {
            GenericPaymentIcon.Visibility = Visibility.Visible;
        }
    }

    private void RenderCodexQuotas(IReadOnlyList<QuotaWindow> windows)
    {
        CodexQuotaPanel.Children.Clear();
        if (windows.Count == 0)
        {
            CodexQuotaPanel.Children.Add(new TextBlock
            {
                Text = "Codex 额度未返回",
                Foreground = (Brush)FindResource("MutedBrush"),
                FontSize = 12,
            });
            return;
        }

        foreach (var quota in windows.Take(2))
        {
            var remaining = quota.UsedPercent is { } used ? Math.Clamp(100d - used, 0d, 100d) : (double?)null;
            var header = new Grid { Margin = new Thickness(0, 0, 0, 4) };
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            header.Children.Add(new TextBlock
            {
                Text = quota.Name,
                FontSize = 11,
                FontWeight = FontWeights.SemiBold,
                TextTrimming = TextTrimming.CharacterEllipsis,
                Margin = new Thickness(0, 0, 10, 0),
            });
            var value = new TextBlock
            {
                Text = remaining is null ? "未返回" : $"剩余 {remaining:0.#}%",
                HorizontalAlignment = HorizontalAlignment.Right,
                FontSize = 11,
                Foreground = quota.Allowed == false ? BrushFrom("#B6503D") : BrushFrom("#3D7A59"),
                FontWeight = FontWeights.SemiBold,
            };
            Grid.SetColumn(value, 1);
            header.Children.Add(value);

            var progress = new ProgressBar
            {
                Height = 5,
                Minimum = 0,
                Maximum = 100,
                Value = remaining ?? 0,
                Background = BrushFrom("#E8E4DE"),
                Foreground = quota.Allowed == false ? BrushFrom("#C96442") : BrushFrom("#638B71"),
            };
            var resetText = quota.ResetsAt?.ToLocalTime().ToString("MM-dd HH:mm", CultureInfo.InvariantCulture) ?? "重置时间未返回";
            var reset = new TextBlock
            {
                Text = resetText,
                Foreground = (Brush)FindResource("MutedBrush"),
                FontSize = 9,
                HorizontalAlignment = HorizontalAlignment.Right,
                Margin = new Thickness(0, 3, 0, 0),
            };
            var container = new StackPanel { Margin = new Thickness(0, 0, 0, 7) };
            container.Children.Add(header);
            container.Children.Add(progress);
            container.Children.Add(reset);
            CodexQuotaPanel.Children.Add(container);
        }
    }

    private void RenderFeatureQuotas(IReadOnlyList<FeatureQuota> quotas)
    {
        FeatureQuotaPanel.Children.Clear();
        foreach (var quota in quotas.Take(4))
        {
            var panel = new StackPanel { Margin = new Thickness(10, 7, 8, 4) };
            panel.Children.Add(new TextBlock
            {
                Text = quota.Name,
                FontSize = 10,
                Foreground = (Brush)FindResource("MutedBrush"),
            });
            panel.Children.Add(new TextBlock
            {
                Text = quota.DisplayValue,
                FontSize = 12,
                FontWeight = FontWeights.SemiBold,
                Margin = new Thickness(0, 3, 0, 0),
                Foreground = quota.Verified ? BrushFrom("#3D7A59") : (Brush)FindResource("InkBrush"),
            });
            panel.ToolTip = quota.Detail;
            var border = new Border
            {
                Background = BrushFrom("#F7F4EF"),
                CornerRadius = new CornerRadius(8),
                Margin = new Thickness(0, 0, 8, 7),
                Child = panel,
            };
            FeatureQuotaPanel.Children.Add(border);
        }
    }

    private void RenderBilling(IReadOnlyList<BillingRecord> records)
    {
        BillingPanel.Children.Clear();
        BillingCountText.Text = records.Count == 0 ? "没有返回网页账单" : $"返回 {records.Count} 条 · 显示最近 4 条";
        if (records.Count == 0)
        {
            BillingPanel.Children.Add(new TextBlock
            {
                Text = "此账户没有可显示的 ChatGPT 网页账单。\nApple / Google 的完整记录不会由该接口返回。",
                Foreground = (Brush)FindResource("MutedBrush"),
                FontSize = 11,
                LineHeight = 19,
                Margin = new Thickness(0, 10, 0, 0),
            });
            return;
        }

        foreach (var record in records.Take(4))
        {
            var grid = new Grid { Height = 44 };
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(92) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            grid.Children.Add(new TextBlock
            {
                Text = record.CreatedAt?.LocalDateTime.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture) ?? "日期未知",
                FontSize = 10,
                Foreground = (Brush)FindResource("MutedBrush"),
                VerticalAlignment = VerticalAlignment.Center,
            });
            var product = new TextBlock
            {
                Text = record.Product,
                FontSize = 11,
                TextTrimming = TextTrimming.CharacterEllipsis,
                VerticalAlignment = VerticalAlignment.Center,
            };
            Grid.SetColumn(product, 1);
            grid.Children.Add(product);
            var amount = new TextBlock
            {
                Text = record.Amount is { } number
                    ? CurrencyRules.Format(number, record.Currency)
                    : LocalizeInvoiceStatus(record.Status),
                FontSize = 11,
                FontWeight = FontWeights.SemiBold,
                VerticalAlignment = VerticalAlignment.Center,
            };
            Grid.SetColumn(amount, 2);
            grid.Children.Add(amount);
            BillingPanel.Children.Add(grid);
        }
    }

    private void ClearButton_Click(object sender, RoutedEventArgs e)
    {
        CredentialBox.Clear();
        ConsentCheckBox.IsChecked = false;
        CredentialBox.Focus();
    }

    private void NewQueryButton_Click(object sender, RoutedEventArgs e)
    {
        ResultPanel.Visibility = Visibility.Collapsed;
        ConnectPanel.Visibility = Visibility.Visible;
        CredentialBox.Focus();
    }

    private void CancelButton_Click(object sender, RoutedEventArgs e) => _queryCancellation?.Cancel();

    private static string FormatRemaining(DateTimeOffset? expiry)
    {
        if (expiry is null)
        {
            return "未返回";
        }

        var remaining = expiry.Value - DateTimeOffset.Now;
        if (remaining <= TimeSpan.Zero)
        {
            return "已到期";
        }

        return remaining.TotalDays >= 1
            ? $"{(int)remaining.TotalDays} 天 {remaining.Hours} 小时"
            : $"{Math.Max(0, remaining.Hours)} 小时 {remaining.Minutes} 分";
    }

    private static string LocalizeInvoiceStatus(string status) => status.ToUpperInvariant() switch
    {
        "PAID" or "SUCCEEDED" or "COMPLETE" => "已支付",
        "REFUNDED" => "已退款",
        "OPEN" or "PENDING" => "待处理",
        "VOID" or "FAILED" => "未支付",
        _ => status,
    };

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _validationTimer.Stop();
        _queryCancellation?.Cancel();
        _queryCancellation?.Dispose();
        _queryCancellation = null;
        _queryService.Dispose();
        GC.SuppressFinalize(this);
    }

    private static SolidColorBrush BrushFrom(string color) =>
        new((Color)ColorConverter.ConvertFromString(color));

    private static IEnumerable<T> FindVisualChildren<T>(DependencyObject dependencyObject) where T : DependencyObject
    {
        for (var index = 0; index < VisualTreeHelper.GetChildrenCount(dependencyObject); index++)
        {
            var child = VisualTreeHelper.GetChild(dependencyObject, index);
            if (child is T match)
            {
                yield return match;
            }

            foreach (var descendant in FindVisualChildren<T>(child))
            {
                yield return descendant;
            }
        }
    }
}
