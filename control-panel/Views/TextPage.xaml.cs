using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;
using System.Linq;

namespace SpaceThumbnails.ControlPanel.Views
{
    public sealed partial class TextPage : Page
    {
        public ObservableCollection<FormatItem> Formats { get; } = new();

        // The GUID for Text Generator, defined in crates/windows/src/constant.rs
        private const string TextGeneratorGuid = "{bb2657d4-0325-4632-9154-116584281364}";

        public TextPage()
        {
            this.InitializeComponent();
            LoadFormats();
        }

        private void LoadFormats()
        {
            var supportedFormats = new[] 
            { 
                (".txt", "Text Document"),
                (".rs", "Rust Source File"),
                (".json", "JSON File"),
                (".xml", "XML Document"),
                (".md", "Markdown File"),
                (".log", "Log File"),
                (".ini", "Configuration File"),
                (".yaml", "YAML Document"),
                (".toml", "TOML Configuration")
            };

            foreach (var (ext, desc) in supportedFormats)
            {
                // Note: We use the same GUID for all text formats
                bool isRegistered = RegistryHelper.IsExtensionRegistered(ext, TextGeneratorGuid);
                Formats.Add(new FormatItem 
                { 
                    Extension = ext, 
                    Description = desc, 
                    Guid = TextGeneratorGuid, 
                    IsEnabled = isRegistered 
                });
            }

            // Set ItemsSource for the ListView
            FormatsList.ItemsSource = Formats;
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
