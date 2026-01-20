using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;

namespace SpaceThumbnails.ControlPanel
{
    public class FormatItem
    {
        public string Extension { get; set; }
        public string Guid { get; set; }
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
            // Sort by extension alphabetically (case-insensitive)
            FormatsList.ItemsSource = formats.OrderBy(f => f.Extension, StringComparer.OrdinalIgnoreCase).ToList();
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
                // Safety check: Only delete if the current provider IS SpaceThumbnails.
                // This prevents accidentally removing other thumbnail handlers (e.g. from 3D Builder)
                // if they are currently active instead of us.
                string keyPath = $"HKEY_CLASSES_ROOT\\{item.Extension}\\shellex\\{{e357fccd-a995-4576-b01f-234630154e96}}";
                string currentValue = GetRegistryDefaultValue(keyPath);

                // Case insensitive comparison of GUIDs
                if (string.Equals(currentValue, item.Guid, StringComparison.OrdinalIgnoreCase))
                {
                    RunRegCommand("delete", keyPath, "/f");
                }
                else
                {
                    StatusText.Text = $"Skipped: SpaceThumbnails is not active for {item.Extension}. Current: {currentValue ?? "None"}";
                }
            }
        }

        private string GetRegistryDefaultValue(string keyPath)
        {
            try
            {
                // Since we can't easily read HKCR directly without elevation issues in some contexts (though reading should be fine),
                // we'll try to use reg query to be consistent with our elevated operations.
                // However, Process.Start redirect output is cleaner.
                ProcessStartInfo psi = new ProcessStartInfo
                {
                    FileName = "reg",
                    Arguments = $"query \"{keyPath}\" /ve", // /ve queries the default value
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    CreateNoWindow = true
                };

                using (var proc = Process.Start(psi))
                {
                    string output = proc.StandardOutput.ReadToEnd();
                    proc.WaitForExit();
                    
                    if (proc.ExitCode == 0)
                    {
                        // Parse output like:
                        // HKEY_CLASSES_ROOT\.obj\shellex\{e357fccd-a995-4576-b01f-234630154e96}
                        //    (Default)    REG_SZ    {650a0a50-3a8c-49ca-ba26-13b31965b8ef}
                        
                        // Simple parse: find the GUID pattern
                        int braceIndex = output.IndexOf('{');
                        if (braceIndex >= 0)
                        {
                            // Find the LAST brace group, which should be the value, not the key name
                            // The key name is on the first line, value on second.
                            string[] lines = output.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
                            foreach(var line in lines)
                            {
                                if (line.Contains("REG_SZ"))
                                {
                                    int valIndex = line.IndexOf("REG_SZ");
                                    string val = line.Substring(valIndex + 6).Trim();
                                    return val;
                                }
                            }
                        }
                    }
                }
            }
            catch { }
            return null;
        }

        private void OnEnableThumbnailClick(object sender, RoutedEventArgs e)
        {
            if (sender is Button btn && btn.Tag is FormatItem item)
            {
                RunRegCommand("add", $"HKEY_CLASSES_ROOT\\{item.Extension}\\shellex\\{{e357fccd-a995-4576-b01f-234630154e96}}", $"/d \"{item.Guid}\" /f");
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
                    // Refresh Explorer using SHChangeNotify instead of restarting explorer.exe
                    // This avoids killing the shell and provides a seamless update.
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
                // Assuming the DLL is in the standard debug path for now, 
                // but for a real app we should find it relative to the executable or in a known install location.
                // We'll try to find it relative to the repo root.
                // Since this app will be in space-thumbnails/control-panel/bin/... 
                // We need to look up.
                
                // Hardcoded path based on previous context for reliability in this specific environment
                // Use Release build DLL
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
                    Verb = "runas" // Request elevation
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
                // Kill explorer
                Process.Start("taskkill", "/f /im explorer.exe").WaitForExit();

                // Delete thumbcache files
                string localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
                string explorerDir = Path.Combine(localAppData, "Microsoft", "Windows", "Explorer");
                
                if (Directory.Exists(explorerDir))
                {
                    var files = Directory.GetFiles(explorerDir, "thumbcache_*.db");
                    foreach (var file in files)
                    {
                        try { File.Delete(file); } catch { /* Ignore locked files */ }
                    }
                }

                // Restart explorer
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
                // Ensure explorer restarts even if delete fails
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
