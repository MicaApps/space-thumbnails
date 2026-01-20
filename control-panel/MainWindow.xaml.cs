using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Data;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.ComponentModel;
using Microsoft.Win32;

namespace SpaceThumbnails.ControlPanel
{
    public class FormatItem : INotifyPropertyChanged
    {
        public string Extension { get; set; }
        public string Guid { get; set; }

        private bool _isEnabled;
        public bool IsEnabled
        {
            get => _isEnabled;
            set
            {
                if (_isEnabled != value)
                {
                    _isEnabled = value;
                    PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(IsEnabled)));
                }
            }
        }

        public event PropertyChangedEventHandler PropertyChanged;
    }

    public class StatusToStyleConverter : IValueConverter
    {
        public Style HighlightStyle { get; set; }
        public Style NormalStyle { get; set; }

        public object Convert(object value, Type targetType, object parameter, string language)
        {
            if (value is bool isEnabled && parameter is string mode)
            {
                bool highlight = false;
                if (mode == "Enable") highlight = isEnabled;
                else if (mode == "Restore") highlight = !isEnabled;

                if (highlight) return HighlightStyle;
            }
            return NormalStyle;
        }

        public object ConvertBack(object value, Type targetType, object parameter, string language)
        {
            throw new NotImplementedException();
        }
    }

    public sealed partial class MainWindow : Window
    {
        public MainWindow()
        {
            this.InitializeComponent();
            this.Title = "Space Thumbnails Control Panel";

            TrySetMicaBackdrop();

            var formats = new List<FormatItem>
            {
                new FormatItem { Extension = ".obj", Guid = "{650a0a50-3a8c-49ca-ba26-13b31965b8ef}" },
                new FormatItem { Extension = ".fbx", Guid = "{bf2644df-ae9c-4524-8bfd-2d531b837e97}" },
                new FormatItem { Extension = ".stl", Guid = "{b9bcfb2d-6dc4-43a0-b161-64ca282a20ff}" },
                new FormatItem { Extension = ".dae", Guid = "{7cacb561-20c5-4b90-bd1c-5aba58b978ca}" },
                new FormatItem { Extension = ".ply", Guid = "{b0225f87-babe-4d50-92a9-37c3c668a3e4}" },
                new FormatItem { Extension = ".x3d", Guid = "{145e37f5-99a1-40f4-b74a-6534524f29ba}" },
                new FormatItem { Extension = ".x3db", Guid = "{1ba6aa5e-ac9a-4d3a-bcd5-678e0669fb27}" },
                new FormatItem { Extension = ".3ds", Guid = "{93c86d4a-6432-43e2-9082-64bdb6cbfa43}" },
                new FormatItem { Extension = ".3mf", Guid = "{442657d4-0325-4632-9154-116584281358}" },
                new FormatItem { Extension = ".stp", Guid = "{552657d4-0325-4632-9154-116584281359}" },
                new FormatItem { Extension = ".step", Guid = "{662657d4-0325-4632-9154-116584281360}" },
                new FormatItem { Extension = ".iges", Guid = "{772657d4-0325-4632-9154-116584281361}" },
                new FormatItem { Extension = ".igs", Guid = "{882657d4-0325-4632-9154-116584281362}" },
                new FormatItem { Extension = ".gltf", Guid = "{d13b767b-a97f-4753-a4a3-7c7c15f6b25c}" },
                new FormatItem { Extension = ".glb", Guid = "{99ff43f0-d914-4a7a-8325-a8013995c41d}" }
            };
            
            foreach(var f in formats)
            {
                UpdateItemStatus(f);
            }

            FormatsList.ItemsSource = formats.OrderBy(f => f.Extension, StringComparer.OrdinalIgnoreCase).ToList();
        }

        private void UpdateItemStatus(FormatItem item)
        {
            try
            {
                bool active = false;
                string guid = item.Guid;
                string thumbnailProviderKey = "\\shellex\\{e357fccd-a995-4576-b01f-234630154e96}";

                // 1. Check Extension
                string extVal = Registry.GetValue($"HKEY_CLASSES_ROOT\\{item.Extension}{thumbnailProviderKey}", "", null) as string;
                if (string.Equals(extVal, guid, StringComparison.OrdinalIgnoreCase)) active = true;

                // 2. Check ProgID
                if (!active)
                {
                    string progId = Registry.GetValue($"HKEY_CLASSES_ROOT\\{item.Extension}", "", null) as string;
                    if (!string.IsNullOrEmpty(progId))
                    {
                        string progVal = Registry.GetValue($"HKEY_CLASSES_ROOT\\{progId}{thumbnailProviderKey}", "", null) as string;
                        if (string.Equals(progVal, guid, StringComparison.OrdinalIgnoreCase)) active = true;
                    }
                }

                // 3. Check SystemFileAssociations
                if (!active)
                {
                    string sysVal = Registry.GetValue($"HKEY_CLASSES_ROOT\\SystemFileAssociations\\{item.Extension}{thumbnailProviderKey}", "", null) as string;
                    if (string.Equals(sysVal, guid, StringComparison.OrdinalIgnoreCase)) active = true;
                }

                item.IsEnabled = active;
            }
            catch
            {
                item.IsEnabled = false;
            }
        }

        private void TrySetMicaBackdrop()
        {
            if (Microsoft.UI.Composition.SystemBackdrops.MicaController.IsSupported())
            {
                this.SystemBackdrop = new MicaBackdrop();
            }
        }

        private void OnRestoreAssociationClick(object sender, RoutedEventArgs e)
        {
            if (sender is Button btn && btn.Tag is FormatItem item)
            {
                string thumbnailProviderKey = "\\shellex\\{e357fccd-a995-4576-b01f-234630154e96}";
                
                // 1. Delete Extension
                string extKey = $"HKEY_CLASSES_ROOT\\{item.Extension}{thumbnailProviderKey}";
                // Only delete if it matches ours, OR if we just want to force clean.
                // Checking value first is safer.
                string extVal = Registry.GetValue(extKey, "", null) as string;
                if (string.Equals(extVal, item.Guid, StringComparison.OrdinalIgnoreCase))
                {
                    RunRegCommand("delete", extKey, "/f");
                }

                // 2. Delete ProgID
                string progId = Registry.GetValue($"HKEY_CLASSES_ROOT\\{item.Extension}", "", null) as string;
                if (!string.IsNullOrEmpty(progId))
                {
                    string progKey = $"HKEY_CLASSES_ROOT\\{progId}{thumbnailProviderKey}";
                    string progVal = Registry.GetValue(progKey, "", null) as string;
                    if (string.Equals(progVal, item.Guid, StringComparison.OrdinalIgnoreCase))
                    {
                        RunRegCommand("delete", progKey, "/f");
                    }
                }

                // 3. Delete SystemFileAssociations
                string sysKey = $"HKEY_CLASSES_ROOT\\SystemFileAssociations\\{item.Extension}{thumbnailProviderKey}";
                string sysVal = Registry.GetValue(sysKey, "", null) as string;
                if (string.Equals(sysVal, item.Guid, StringComparison.OrdinalIgnoreCase))
                {
                    RunRegCommand("delete", sysKey, "/f");
                }

                UpdateItemStatus(item);
            }
        }

        private void OnEnableThumbnailClick(object sender, RoutedEventArgs e)
        {
            if (sender is Button btn && btn.Tag is FormatItem item)
            {
                RunRegCommand("add", $"HKEY_CLASSES_ROOT\\{item.Extension}\\shellex\\{{e357fccd-a995-4576-b01f-234630154e96}}", $"/d \"{item.Guid}\" /f");
                UpdateItemStatus(item);
            }
        }

        [System.Runtime.InteropServices.DllImport("shell32.dll")]
        private static extern void SHChangeNotify(int wEventId, int uFlags, IntPtr dwItem1, IntPtr dwItem2);

        private const int SHCNE_ASSOCCHANGED = 0x08000000;
        private const int SHCNF_IDLIST = 0x0000;

        private void RunRegCommand(string operation, string key, string args)
        {
            try
            {
                ProcessStartInfo psi = new ProcessStartInfo
                {
                    FileName = "reg",
                    Arguments = $"{operation} \"{key}\" {args}",
                    UseShellExecute = true,
                    Verb = "runas",
                    WindowStyle = ProcessWindowStyle.Hidden
                };

                var proc = Process.Start(psi);
                proc.WaitForExit();
                
                if (proc.ExitCode == 0)
                {
                    StatusText.Text = $"Success: {operation} {key}";
                    SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, IntPtr.Zero, IntPtr.Zero);
                }
                else
                {
                    StatusText.Text = $"Failed (Exit Code {proc.ExitCode}): {operation} {key}";
                }
            }
            catch (Exception ex)
            {
                StatusText.Text = $"Error: {ex.Message}";
            }
        }

        private void OnApplyThumbnailsClick(object sender, RoutedEventArgs e)
        {
            try 
            {
                string dllPath = @"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\target\release\space_thumbnails_windows_dll.dll";
                
                if (!File.Exists(dllPath))
                {
                    StatusText.Text = $"Error: DLL not found at {dllPath}";
                    return;
                }

                ProcessStartInfo psi = new ProcessStartInfo
                {
                    FileName = "regsvr32",
                    Arguments = $"/s \"{dllPath}\"",
                    UseShellExecute = true,
                    Verb = "runas"
                };

                Process.Start(psi);
                StatusText.Text = "Registration command executed.";
            }
            catch (Exception ex)
            {
                StatusText.Text = $"Error: {ex.Message}";
            }
        }

        private async void RebuildIconCache_Click(object sender, RoutedEventArgs e)
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
                    Title = "成功",
                    Content = "图标缓存已重建！",
                    CloseButtonText = "确定",
                    XamlRoot = this.Content.XamlRoot
                };
                await dialog.ShowAsync();
            }
            catch (Exception ex)
            {
                try { Process.Start("explorer.exe"); } catch { }

                ContentDialog dialog = new ContentDialog
                {
                    Title = "错误",
                    Content = $"重建缓存失败: {ex.Message}",
                    CloseButtonText = "确定",
                    XamlRoot = this.Content.XamlRoot
                };
                await dialog.ShowAsync();
            }
        }
    }
}