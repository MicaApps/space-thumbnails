using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media.Imaging;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.ComponentModel;
using System.Threading.Tasks;
using Microsoft.Win32;
using Microsoft.Windows.ApplicationModel.Resources;
using Windows.Storage;
using Windows.Storage.FileProperties;

namespace SpaceThumbnails.ControlPanel
{
    public class FormatItem : INotifyPropertyChanged
    {
        public string Extension { get; set; }
        public string Guid { get; set; }
        public string Category { get; set; } // "3d" or "images"

        private ImageSource _previewImage;
        public ImageSource PreviewImage
        {
            get => _previewImage;
            set
            {
                if (_previewImage != value)
                {
                    _previewImage = value;
                    PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(PreviewImage)));
                    PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(ShowPreview)));
                    PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(ShowIcon)));
                }
            }
        }

        public Visibility ShowPreview => _previewImage != null ? Visibility.Visible : Visibility.Collapsed;
        public Visibility ShowIcon => _previewImage == null ? Visibility.Visible : Visibility.Collapsed;

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
        private List<FormatItem> _allFormats;
        private readonly ResourceLoader _resourceLoader = new ResourceLoader();

        public MainWindow()
        {
            this.InitializeComponent();
            
            this.Title = _resourceLoader.GetString("AppTitle/Text");

            TrySetMicaBackdrop();

            _allFormats = new List<FormatItem>
            {
                // 3D Models
                new FormatItem { Extension = ".obj", Guid = "{650a0a50-3a8c-49ca-ba26-13b31965b8ef}", Category = "3d" },
                new FormatItem { Extension = ".fbx", Guid = "{bf2644df-ae9c-4524-8bfd-2d531b837e97}", Category = "3d" },
                new FormatItem { Extension = ".stl", Guid = "{b9bcfb2d-6dc4-43a0-b161-64ca282a20ff}", Category = "3d" },
                new FormatItem { Extension = ".dae", Guid = "{7cacb561-20c5-4b90-bd1c-5aba58b978ca}", Category = "3d" },
                new FormatItem { Extension = ".ply", Guid = "{b0225f87-babe-4d50-92a9-37c3c668a3e4}", Category = "3d" },
                new FormatItem { Extension = ".x3d", Guid = "{145e37f5-99a1-40f4-b74a-6534524f29ba}", Category = "3d" },
                new FormatItem { Extension = ".x3db", Guid = "{1ba6aa5e-ac9a-4d3a-bcd5-678e0669fb27}", Category = "3d" },
                new FormatItem { Extension = ".3ds", Guid = "{93c86d4a-6432-43e2-9082-64bdb6cbfa43}", Category = "3d" },
                new FormatItem { Extension = ".3mf", Guid = "{442657d4-0325-4632-9154-116584281358}", Category = "3d" },
                new FormatItem { Extension = ".stp", Guid = "{552657d4-0325-4632-9154-116584281359}", Category = "3d" },
                new FormatItem { Extension = ".step", Guid = "{662657d4-0325-4632-9154-116584281360}", Category = "3d" },
                new FormatItem { Extension = ".iges", Guid = "{772657d4-0325-4632-9154-116584281361}", Category = "3d" },
                new FormatItem { Extension = ".igs", Guid = "{882657d4-0325-4632-9154-116584281362}", Category = "3d" },
                new FormatItem { Extension = ".gltf", Guid = "{d13b767b-a97f-4753-a4a3-7c7c15f6b25c}", Category = "3d" },
                new FormatItem { Extension = ".glb", Guid = "{99ff43f0-d914-4a7a-8325-a8013995c41d}", Category = "3d" },
                
                // Images
                new FormatItem { Extension = ".psd", Guid = "{905657D4-0325-4632-9154-116584281399}", Category = "images" },

                // Books
                new FormatItem { Extension = ".epub", Guid = "{772657D4-0325-4632-9154-116584281388}", Category = "books" },

                // Documents
                new FormatItem { Extension = ".pdf", Guid = "{102657D4-0325-4632-9154-116584281399}", Category = "document" },
                new FormatItem { Extension = ".docx", Guid = "{442657D4-0325-4632-9154-116584281373}", Category = "document" },
                new FormatItem { Extension = ".xlsx", Guid = "{992657D4-0325-4632-9154-116584281358}", Category = "document" },
                new FormatItem { Extension = ".pptx", Guid = "{992657D4-0325-4632-9154-116584281359}", Category = "document" },
                new FormatItem { Extension = ".doc", Guid = "{992657D4-0325-4632-9154-116584281367}", Category = "document" },
                new FormatItem { Extension = ".xls", Guid = "{992657D4-0325-4632-9154-116584281368}", Category = "document" },
                new FormatItem { Extension = ".ppt", Guid = "{992657D4-0325-4632-9154-116584281369}", Category = "document" },
                new FormatItem { Extension = ".pages", Guid = "{992657D4-0325-4632-9154-116584281370}", Category = "document" },
                new FormatItem { Extension = ".numbers", Guid = "{992657D4-0325-4632-9154-116584281371}", Category = "document" },
                new FormatItem { Extension = ".key", Guid = "{992657D4-0325-4632-9154-116584281372}", Category = "document" }
            };
            
            foreach(var f in _allFormats)
            {
                UpdateItemStatus(f);
            }
            
            LoadPreviews();
        }

        private async void LoadPreviews()
        {
            string samplesPath = Path.Combine(AppContext.BaseDirectory, "Samples");
            if (!Directory.Exists(samplesPath)) return;

            foreach (var item in _allFormats)
            {
                try
                {
                    string fileName = "sample" + item.Extension;
                    string filePath = Path.Combine(samplesPath, fileName);
                    
                    if (!File.Exists(filePath))
                    {
                        // Try capitalized version
                        fileName = "Sample" + item.Extension;
                        filePath = Path.Combine(samplesPath, fileName);
                    }
                    
                    if (File.Exists(filePath))
                    {
                        StorageFile file = await StorageFile.GetFileFromPathAsync(filePath);
                        var thumbnail = await file.GetThumbnailAsync(ThumbnailMode.SingleItem, 64);
                        if (thumbnail != null)
                        {
                            BitmapImage bmp = new BitmapImage();
                            await bmp.SetSourceAsync(thumbnail);
                            item.PreviewImage = bmp;
                        }
                    }
                }
                catch { }
            }
        }

        private void NavView_Loaded(object sender, RoutedEventArgs e)
        {
            // Select the first item ("3D Models") by default
            if (NavView.MenuItems.Count > 0)
            {
                NavView.SelectedItem = NavView.MenuItems[0];
            }
        }

        private void NavView_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
        {
            if (args.IsSettingsSelected)
            {
                // Not implemented yet
                FormatsList.ItemsSource = null;
                return;
            }

            if (args.SelectedItem is NavigationViewItem selectedItem)
            {
                string tag = selectedItem.Tag?.ToString();
                FilterList(tag);
            }
        }

        private void FilterList(string category)
        {
            if (_allFormats == null) return;

            var filtered = _allFormats
                .Where(f => string.Equals(f.Category, category, StringComparison.OrdinalIgnoreCase))
                .OrderBy(f => f.Extension, StringComparer.OrdinalIgnoreCase)
                .ToList();
            
            FormatsList.ItemsSource = filtered;
        }

        private string GetRegistryValue64(string subKeyPath, string valueName)
        {
            // Check HKCU first (User 64-bit)
            string val = GetRegistryValue64(RegistryHive.CurrentUser, subKeyPath, valueName);
            if (val != null) return val;

            // Check HKLM second (System 64-bit)
            return GetRegistryValue64(RegistryHive.LocalMachine, subKeyPath, valueName);
        }

        private string GetRegistryValue64(RegistryHive hive, string subKeyPath, string valueName)
        {
            try
            {
                using (var baseKey = RegistryKey.OpenBaseKey(hive, RegistryView.Registry64))
                using (var key = baseKey.OpenSubKey($"Software\\Classes\\{subKeyPath}"))
                {
                    if (key != null)
                    {
                        var val = key.GetValue(valueName);
                        if (val != null) return val.ToString();
                    }
                }
            }
            catch { }
            return null;
        }

        private void UpdateItemStatus(FormatItem item)
        {
            try
            {
                bool active = false;
                string guid = item.Guid;
                // Remove the leading slash from the key path for OpenSubKey
                string thumbnailProviderSubKey = $"shellex\\{{e357fccd-a995-4576-b01f-234630154e96}}";

                // 1. Check Extension
                // HKEY_CLASSES_ROOT\.ext\shellex\... -> Software\Classes\.ext\shellex\...
                string extVal = GetRegistryValue64($"{item.Extension}\\{thumbnailProviderSubKey}", "");
                if (string.Equals(extVal, guid, StringComparison.OrdinalIgnoreCase)) active = true;

                // 2. Check ProgID
                if (!active)
                {
                    // Get ProgID from extension
                    string progId = GetRegistryValue64(item.Extension, "");
                    if (!string.IsNullOrEmpty(progId))
                    {
                        string progVal = GetRegistryValue64($"{progId}\\{thumbnailProviderSubKey}", "");
                        if (string.Equals(progVal, guid, StringComparison.OrdinalIgnoreCase)) active = true;
                    }
                }

                // 3. Check SystemFileAssociations
                if (!active)
                {
                    string sysVal = GetRegistryValue64($"SystemFileAssociations\\{item.Extension}\\{thumbnailProviderSubKey}", "");
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

        private void CleanRegistration(string relativePath, string subKey, string targetGuid)
        {
            // relativePath: e.g. ".step" or "stp_auto_file"
            // subKey: e.g. "shellex\{...}"
            
            // We explicit check both HKCU and HKLM to ensure no residue is left.
            var hives = new[] { RegistryHive.CurrentUser, RegistryHive.LocalMachine };
            var rootNames = new[] { "HKEY_CURRENT_USER", "HKEY_LOCAL_MACHINE" };

            for (int i = 0; i < hives.Length; i++)
            {
                string fullPath = $"{relativePath}\\{subKey}";
                // Check if the key exists and matches our GUID using 64-bit view
                string val = GetRegistryValue64(hives[i], fullPath, "");
                
                if (string.Equals(val, targetGuid, StringComparison.OrdinalIgnoreCase))
                {
                    // Found it! Delete it.
                    // Construct the full key path for 'reg' command
                    string regKeyPath = $"{rootNames[i]}\\Software\\Classes\\{fullPath}";
                    RunRegCommand("delete", regKeyPath, "/f");
                }
            }
        }

        private void OnRestoreAssociationClick(object sender, RoutedEventArgs e)
        {
            if (sender is Button btn && btn.Tag is FormatItem item)
            {
                string thumbnailProviderKey = "shellex\\{e357fccd-a995-4576-b01f-234630154e96}";
                
                // 1. Clean Extension (.step)
                CleanRegistration(item.Extension, thumbnailProviderKey, item.Guid);

                // 2. Clean ProgID (e.g. stp_auto_file)
                string progId = GetRegistryValue64(item.Extension, "");
                if (!string.IsNullOrEmpty(progId))
                {
                    CleanRegistration(progId, thumbnailProviderKey, item.Guid);
                }

                // 3. Clean SystemFileAssociations
                CleanRegistration($"SystemFileAssociations\\{item.Extension}", thumbnailProviderKey, item.Guid);

                UpdateItemStatus(item);
            }
        }

        private void OnEnableThumbnailClick(object sender, RoutedEventArgs e)
        {
            if (sender is Button btn && btn.Tag is FormatItem item)
            {
                string thumbnailProviderKey = "shellex\\{e357fccd-a995-4576-b01f-234630154e96}";
                
                // 1. Register to extension
                RunRegCommand("add", $"HKEY_CLASSES_ROOT\\{item.Extension}\\{thumbnailProviderKey}", $"/d \"{item.Guid}\" /f");
                
                // 2. Register to ProgID if exists
                string progId = GetRegistryValue64(item.Extension, "");
                if (!string.IsNullOrEmpty(progId) && !progId.StartsWith("{"))
                {
                    RunRegCommand("add", $"HKEY_CLASSES_ROOT\\{progId}\\{thumbnailProviderKey}", $"/d \"{item.Guid}\" /f");
                }

                // 3. Register to SystemFileAssociations
                RunRegCommand("add", $"HKEY_CLASSES_ROOT\\SystemFileAssociations\\{item.Extension}\\{thumbnailProviderKey}", $"/d \"{item.Guid}\" /f");

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
                    StatusText.Text = string.Format(_resourceLoader.GetString("Msg_Success"), operation, key);
                    SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, IntPtr.Zero, IntPtr.Zero);
                }
                else
                {
                    StatusText.Text = string.Format(_resourceLoader.GetString("Msg_Failed"), proc.ExitCode, operation, key);
                }
            }
            catch (Exception ex)
            {
                StatusText.Text = string.Format(_resourceLoader.GetString("Msg_Error"), ex.Message);
            }
        }

        private void OnApplyThumbnailsClick(object sender, RoutedEventArgs e)
        {
            try 
            {
                // 1. Production/Packaged Mode: Check current directory
                string appDir = AppDomain.CurrentDomain.BaseDirectory;
                string dllName = "space_thumbnails_windows.dll";
                string dllPath = Path.Combine(appDir, dllName);

                // 2. Dev Mode (VS Output): Check if we are in bin/... and DLL is in project root or target
                if (!File.Exists(dllPath))
                {
                    // Fallback to hardcoded dev path for convenience during development
                    string devPath = @"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target\release\space_thumbnails_windows.dll";
                    if (File.Exists(devPath))
                    {
                        dllPath = devPath;
                    }
                }

                if (!File.Exists(dllPath))
                {
                    StatusText.Text = string.Format(_resourceLoader.GetString("Msg_DllNotFound"), dllPath);
                    return;
                }

                ProcessStartInfo psi = new ProcessStartInfo
                {
                    FileName = "regsvr32",
                    Arguments = $"/s \"{dllPath}\"",
                    UseShellExecute = true,
                    Verb = "runas"
                };

                var proc = Process.Start(psi);
                if (proc != null)
                {
                    proc.WaitForExit();
                    StatusText.Text = _resourceLoader.GetString("Msg_RegCommandExecuted");
                }
            }
            catch (Exception ex)
            {
                StatusText.Text = string.Format(_resourceLoader.GetString("Msg_Error"), ex.Message);
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

                // Also clear our own app's thumbnail cache (New path: %TEMP%\SpaceThumbnailsCache)
                string appCacheDir = Path.Combine(Path.GetTempPath(), "SpaceThumbnailsCache");
                if (Directory.Exists(appCacheDir))
                {
                    // Clean recursively
                    Directory.Delete(appCacheDir, true);
                }

                Process.Start("explorer.exe");
                
                ContentDialog dialog = new ContentDialog
                {
                    Title = _resourceLoader.GetString("Msg_RebuildSuccess_Title"),
                    Content = _resourceLoader.GetString("Msg_RebuildSuccess_Content"),
                    CloseButtonText = _resourceLoader.GetString("Btn_OK"),
                    XamlRoot = this.Content.XamlRoot
                };
                await dialog.ShowAsync();
            }
            catch (Exception ex)
            {
                try { Process.Start("explorer.exe"); } catch { }

                ContentDialog dialog = new ContentDialog
                {
                    Title = _resourceLoader.GetString("Msg_RebuildError_Title"),
                    Content = string.Format(_resourceLoader.GetString("Msg_RebuildError_Content"), ex.Message),
                    CloseButtonText = _resourceLoader.GetString("Btn_OK"),
                    XamlRoot = this.Content.XamlRoot
                };
                await dialog.ShowAsync();
            }
        }
    }
}