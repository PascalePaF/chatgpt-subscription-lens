using System.Windows;
using SubscriptionLens.Core;

namespace SubscriptionLens.App;

public partial class SubscriptionLensApplication : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        ArgumentNullException.ThrowIfNull(e);
        if (e.Args.Contains("--self-test", StringComparer.OrdinalIgnoreCase))
        {
            var validation = CredentialParser.Validate("not-a-complete-session");
            Environment.ExitCode = validation.IsValid ? 2 : 0;
            Shutdown(Environment.ExitCode);
            return;
        }

        base.OnStartup(e);
        var window = new MainWindow();
        MainWindow = window;
        if (e.Args.Contains("--ui-self-test", StringComparer.OrdinalIgnoreCase))
        {
            window.Loaded += (_, _) =>
            {
                Environment.ExitCode = window.ValidateLayoutContract() ? 0 : 3;
                window.Close();
                Shutdown(Environment.ExitCode);
            };
        }
        window.Show();
    }
}
