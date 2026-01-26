using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;
using System.Linq;

namespace SpaceThumbnails.ControlPanel.Views
{
    public sealed partial class BooksPage : Page
    {
        public ObservableCollection<FormatItem> Formats { get; } = new();

        // GUIDs defined in crates/windows/src/constant.rs
        private const string EpubGeneratorGuid = "{ee2657d4-0325-4632-9154-116584281367}";
        // PDF GUID
        private const string PdfGeneratorGuid = "{ff2657d4-0325-4632-9154-116584281368}"; 

        public BooksPage()
        {
            this.InitializeComponent();
            LoadFormats();
        }

        private void LoadFormats()
        {
            var supportedFormats = new[] 
            { 
                (".pdf", "Portable Document Format", PdfGeneratorGuid),
                (".epub", "EPUB E-Book", EpubGeneratorGuid)
            };

            foreach (var (ext, desc, guid) in supportedFormats)
            {
                bool isRegistered = RegistryHelper.IsExtensionRegistered(ext, guid);
                Formats.Add(new FormatItem 
                { 
                    Extension = ext, 
                    Description = desc, 
                    Guid = guid, 
                    IsEnabled = isRegistered 
                });
            }

            // Set ItemsSource for the ListView
            FormatsList.ItemsSource = Formats;
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

        private void OnEnableAllClick(object sender, RoutedEventArgs e)
        {
            foreach (var item in Formats)
            {
                if (!item.IsEnabled)
                {
                    if (RegistryHelper.RegisterExtension(item.Extension, item.Guid))
                    {
                        item.IsEnabled = true;
                    }
                }
            }
        }

        private void OnDisableAllClick(object sender, RoutedEventArgs e)
        {
            foreach (var item in Formats)
            {
                if (item.IsEnabled)
                {
                    if (RegistryHelper.UnregisterExtension(item.Extension, item.Guid))
                    {
                        item.IsEnabled = false;
                    }
                }
            }
        }
    }
}
