using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System;
using System.Collections.Generic;
using System.Linq;

namespace SpaceThumbnails.ControlPanel.Views
{
    public sealed partial class ThreeDPage : Page
    {
        public ThreeDPage()
        {
            this.InitializeComponent();
            LoadFormats();
        }

        private void LoadFormats()
        {
            var formats = new List<FormatItem>
            {
                new FormatItem { Extension = ".obj", Description = "Wavefront Object", Guid = "{650a0a50-3a8c-49ca-ba26-13b31965b8ef}" },
                new FormatItem { Extension = ".fbx", Description = "Filmbox", Guid = "{bf2644df-ae9c-4524-8bfd-2d531b837e97}" },
                new FormatItem { Extension = ".stl", Description = "Stereolithography", Guid = "{b9bcfb2d-6dc4-43a0-b161-64ca282a20ff}" },
                new FormatItem { Extension = ".dae", Description = "Collada", Guid = "{7cacb561-20c5-4b90-bd1c-5aba58b978ca}" },
                new FormatItem { Extension = ".ply", Description = "Polygon File Format", Guid = "{b0225f87-babe-4d50-92a9-37c3c668a3e4}" },
                new FormatItem { Extension = ".x3d", Description = "X3D", Guid = "{145e37f5-99a1-40f4-b74a-6534524f29ba}" },
                new FormatItem { Extension = ".x3db", Description = "X3D Binary", Guid = "{1ba6aa5e-ac9a-4d3a-bcd5-678e0669fb27}" },
                new FormatItem { Extension = ".3ds", Description = "3D Studio", Guid = "{93c86d4a-6432-43e2-9082-64bdb6cbfa43}" },
                new FormatItem { Extension = ".3mf", Description = "3D Manufacturing Format", Guid = "{442657d4-0325-4632-9154-116584281358}" },
                new FormatItem { Extension = ".stp", Description = "STEP File", Guid = "{552657d4-0325-4632-9154-116584281359}" },
                new FormatItem { Extension = ".step", Description = "STEP File", Guid = "{662657d4-0325-4632-9154-116584281360}" },
                new FormatItem { Extension = ".iges", Description = "IGES File", Guid = "{772657d4-0325-4632-9154-116584281361}" },
                new FormatItem { Extension = ".igs", Description = "IGES File", Guid = "{882657d4-0325-4632-9154-116584281362}" },
                new FormatItem { Extension = ".gltf", Description = "GL Transmission Format", Guid = "{d13b767b-a97f-4753-a4a3-7c7c15f6b25c}" },
                new FormatItem { Extension = ".glb", Description = "GL Transmission Format Binary", Guid = "{99ff43f0-d914-4a7a-8325-a8013995c41d}" }
            };

            foreach (var f in formats)
            {
                f.IsEnabled = RegistryHelper.IsExtensionRegistered(f.Extension, f.Guid);
                f.PreviewImage = $"ms-appx:///Assets/Previews/{f.Extension.TrimStart('.')}.png";
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
