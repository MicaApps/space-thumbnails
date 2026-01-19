using Microsoft.UI.Xaml;
using System;
using System.Runtime.InteropServices;

namespace SpaceThumbnails.ControlPanel
{
    public partial class App : Application
    {
        public App()
        {
            this.InitializeComponent();
        }

        protected override void OnLaunched(Microsoft.UI.Xaml.LaunchActivatedEventArgs args)
        {
            m_window = new MainWindow();
            m_window.Activate();
        }

        private Window m_window;
    }

    public static class Program
    {
        [DllImport("Microsoft.UI.Xaml.dll")]
        private static extern void XamlCheckProcessRequirements();

        [STAThread]
        static void Main(string[] args)
        {
            try
            {
                XamlCheckProcessRequirements();
                
                WinRT.ComWrappersSupport.InitializeComWrappers();
                Microsoft.UI.Xaml.Application.Start((p) => {
                    var context = new Microsoft.UI.Dispatching.DispatcherQueueSynchronizationContext(Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread());
                    System.Threading.SynchronizationContext.SetSynchronizationContext(context);
                    new App();
                });
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"FATAL CRASH: {ex}");
            }
        }
    }
}
