using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using System;
using System.Linq;
using System.IO;
using System.Diagnostics;
using SpaceThumbnails.ControlPanel.Views;

namespace SpaceThumbnails.ControlPanel
{
    public sealed partial class MainWindow : Window
    {
        public MainWindow()
        {
            this.InitializeComponent();
            this.Title = "Space Thumbnails Control Panel";
            TrySetMicaBackdrop();
        }

        private void TrySetMicaBackdrop()
        {
            if (Microsoft.UI.Composition.SystemBackdrops.MicaController.IsSupported())
            {
                this.SystemBackdrop = new MicaBackdrop();
            }
        }

        private void NavView_Loaded(object sender, RoutedEventArgs e)
        {
            // Select the first item by default
            NavView.SelectedItem = NavView.MenuItems.OfType<NavigationViewItem>().First();
            NavView_Navigate("ThreeDPage");
        }

        private void NavView_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
        {
            if (args.IsSettingsSelected)
            {
                // We handle Settings manually in Footer, so this might not be needed depending on implementation
                // But if we used the built-in SettingsItem:
                // NavView_Navigate("SettingsPage");
            }
            else if (args.SelectedItemContainer != null)
            {
                string navItemTag = args.SelectedItemContainer.Tag.ToString();
                NavView_Navigate(navItemTag);
            }
        }

        private void NavView_Navigate(string navItemTag)
        {
            Type pageType = null;
            switch (navItemTag)
            {
                case "ThreeDPage":
                    pageType = typeof(ThreeDPage);
                    break;
                case "PsdPage":
                    pageType = typeof(PsdPage);
                    break;
                case "TextPage":
                    pageType = typeof(TextPage);
                    break;
                case "BooksPage":
                    pageType = typeof(BooksPage);
                    break;
                //case "GeneralPage":
                //    pageType = typeof(GeneralPage);
                //    break;
            }

            if (pageType != null)
            {
                ContentFrame.Navigate(pageType, null, new Microsoft.UI.Xaml.Media.Animation.DrillInNavigationTransitionInfo());
            }
        }

        private void OnApplyThumbnailsClick(object sender, RoutedEventArgs e)
        {
            try 
            {
                string dllPath = @"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\target\release\space_thumbnails_windows_dll.dll";
                RegistryHelper.RegisterDll(dllPath);
                
                ContentDialog dialog = new ContentDialog
                {
                    Title = "Success",
                    Content = "DLL registered successfully.",
                    CloseButtonText = "OK",
                    XamlRoot = this.Content.XamlRoot
                };
                _ = dialog.ShowAsync();
            }
            catch (Exception ex)
            {
                ContentDialog dialog = new ContentDialog
                {
                    Title = "Error",
                    Content = ex.Message,
                    CloseButtonText = "OK",
                    XamlRoot = this.Content.XamlRoot
                };
                _ = dialog.ShowAsync();
            }
        }

        private async void OnRebuildIconCacheClick(object sender, RoutedEventArgs e)
        {
            try
            {
                Process.Start("taskkill", "/f /im explorer.exe").WaitForExit();

                string localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
                string explorerDir = Path.Combine(localAppData, "Microsoft", "Windows", "Explorer");
                
                if (Directory.Exists(explorerDir))
                {
                    var files = Directory.GetFiles(explorerDir, "thumbcache_*.db");
                    foreach (var file in files)
                    {
                        try { File.Delete(file); } catch { }
                    }
                }

                Process.Start("explorer.exe");
                
                ContentDialog dialog = new ContentDialog
                {
                    Title = "Success",
                    Content = "Icon cache rebuilt!",
                    CloseButtonText = "OK",
                    XamlRoot = this.Content.XamlRoot
                };
                await dialog.ShowAsync();
            }
            catch (Exception ex)
            {
                try { Process.Start("explorer.exe"); } catch { }

                ContentDialog dialog = new ContentDialog
                {
                    Title = "Error",
                    Content = $"Failed to rebuild cache: {ex.Message}",
                    CloseButtonText = "OK",
                    XamlRoot = this.Content.XamlRoot
                };
                await dialog.ShowAsync();
            }
        }
    }
}
