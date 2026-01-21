using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System;
using System.Collections.Generic;
using System.Linq;

namespace SpaceThumbnails.ControlPanel.Views
{
    public sealed partial class PsdPage : Page
    {
        public PsdPage()
        {
            this.InitializeComponent();
            LoadFormats();
        }

        private void LoadFormats()
        {
            var formats = new List<FormatItem>
            {
                new FormatItem { Extension = ".psd", Description = "Adobe Photoshop Document", Guid = "{aa2657d4-0325-4632-9154-116584281363}" }
            };

            foreach (var f in formats)
            {
                f.IsEnabled = RegistryHelper.IsExtensionRegistered(f.Extension, f.Guid);
            }

            FormatsList.ItemsSource = formats.OrderBy(f => f.Extension, StringComparer.OrdinalIgnoreCase).ToList();
        }

        private void OnRestoreAssociationClick(object sender, RoutedEventArgs e)
        {
            if (sender is Button btn && btn.Tag is FormatItem item)
            {
                if (RegistryHelper.UnregisterExtension(item.Extension, item.Guid))
                {
                    item.IsEnabled = false;
                }
            }
        }

        private void OnEnableThumbnailClick(object sender, RoutedEventArgs e)
        {
            if (sender is Button btn && btn.Tag is FormatItem item)
            {
                if (RegistryHelper.RegisterExtension(item.Extension, item.Guid))
                {
                    item.IsEnabled = true;
                }
            }
        }
    }
}
