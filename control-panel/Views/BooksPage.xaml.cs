using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;
using System.Linq;
using System;
using System.ComponentModel;
using System.Threading.Tasks;
using System.Net.Http;
using System.IO;
using System.Diagnostics;

namespace SpaceThumbnails.ControlPanel.Views
{
    public sealed partial class BooksPage : Page, INotifyPropertyChanged
    {
        public ObservableCollection<FormatItem> Formats { get; } = new();

        // GUIDs defined in crates/windows/src/constant.rs
        private const string EpubGeneratorGuid = "{ee2657d4-0325-4632-9154-116584281367}";
        private const string PdfGeneratorGuid = "{ff2657d4-0325-4632-9154-116584281368}"; 
        private const string DocxGeneratorGuid = "{442657d4-0325-4632-9154-116584281373}";

        public event PropertyChangedEventHandler PropertyChanged;
        private void OnPropertyChanged(string name) => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));

        private bool _isLibreOfficeMissing;
        public bool IsLibreOfficeMissing 
        { 
            get => _isLibreOfficeMissing;
            set { _isLibreOfficeMissing = value; OnPropertyChanged(nameof(IsLibreOfficeMissing)); }
        }

        private Visibility _downloadButtonVisibility = Visibility.Visible;
        public Visibility DownloadButtonVisibility
        {
            get => _downloadButtonVisibility;
            set { _downloadButtonVisibility = value; OnPropertyChanged(nameof(DownloadButtonVisibility)); }
        }

        private Visibility _downloadProgressVisibility = Visibility.Collapsed;
        public Visibility DownloadProgressVisibility
        {
            get => _downloadProgressVisibility;
            set { _downloadProgressVisibility = value; OnPropertyChanged(nameof(DownloadProgressVisibility)); }
        }

        private string _downloadStatusText = "";
        public string DownloadStatusText
        {
            get => _downloadStatusText;
            set { _downloadStatusText = value; OnPropertyChanged(nameof(DownloadStatusText)); }
        }

        private double _downloadProgressValue;
        public double DownloadProgressValue
        {
            get => _downloadProgressValue;
            set { _downloadProgressValue = value; OnPropertyChanged(nameof(DownloadProgressValue)); }
        }

        public BooksPage()
        {
            this.InitializeComponent();
            CheckLibreOffice();
            LoadFormats();
        }

        private void CheckLibreOffice()
        {
            // Standard paths
            bool exists = File.Exists(@"C:\Program Files\LibreOffice\program\soffice.exe") ||
                          File.Exists(@"C:\Program Files (x86)\LibreOffice\program\soffice.exe");
            
            // Portable path
            var appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
            var portablePath = Path.Combine(appData, "SpaceThumbnails", "deps", "LibreOfficePortable", "App", "libreoffice", "program", "soffice.exe");
            if (File.Exists(portablePath)) exists = true;

            bool hasOffice = CheckOfficeInstalled();

            IsLibreOfficeMissing = !exists && !hasOffice;
        }

        private bool CheckOfficeInstalled()
        {
            try
            {
                using var key = Microsoft.Win32.Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\Winword.exe");
                return key != null;
            }
            catch
            {
                return false;
            }
        }

        private void LoadFormats()
        {
            var supportedFormats = new[] 
            { 
                (".pdf", "Portable Document Format", PdfGeneratorGuid),
                (".epub", "EPUB E-Book", EpubGeneratorGuid),
                (".docx", "Microsoft Word Document", DocxGeneratorGuid)
            };

            foreach (var (ext, desc, guid) in supportedFormats)
            {
                bool isRegistered = RegistryHelper.IsExtensionRegistered(ext, guid);
                Formats.Add(new FormatItem 
                { 
                    Extension = ext, 
                    Description = desc, 
                    Guid = guid, 
                    IsEnabled = isRegistered,
                    PreviewImage = $"ms-appx:///Assets/Previews/{ext.TrimStart('.')}.png"
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

        private async void OnDownloadLibreOfficeClick(object sender, RoutedEventArgs e)
        {
            try
            {
                DownloadButtonVisibility = Visibility.Collapsed;
                DownloadProgressVisibility = Visibility.Visible;
                DownloadStatusText = "Downloading LibreOffice Portable...";
                DownloadProgressValue = 0;

                // SourceForge link for 7.6.5 Portable
                var downloadUrl = "https://sourceforge.net/projects/portableapps/files/LibreOffice%20Portable/LibreOfficePortable_7.6.5_MultilingualStandard.paf.exe/download";
                var tempFile = Path.Combine(Path.GetTempPath(), "LibreOfficePortable_Installer.exe");

                using (var client = new HttpClient())
                {
                    // Use a browser-like user agent just in case
                    client.DefaultRequestHeaders.UserAgent.ParseAdd("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
                    
                    using (var response = await client.GetAsync(downloadUrl, HttpCompletionOption.ResponseHeadersRead))
                    {
                        response.EnsureSuccessStatusCode();
                        var totalBytes = response.Content.Headers.ContentLength ?? 180_000_000; // Approx 180MB if unknown
                        
                        using (var contentStream = await response.Content.ReadAsStreamAsync())
                        using (var fileStream = new FileStream(tempFile, FileMode.Create, FileAccess.Write, FileShare.None))
                        {
                            var buffer = new byte[8192];
                            var totalRead = 0L;
                            int bytesRead;
                            while ((bytesRead = await contentStream.ReadAsync(buffer, 0, buffer.Length)) > 0)
                            {
                                await fileStream.WriteAsync(buffer, 0, bytesRead);
                                totalRead += bytesRead;
                                DownloadProgressValue = (double)totalRead / totalBytes * 100;
                            }
                        }
                    }
                }

                DownloadStatusText = "Installing...";
                DownloadProgressValue = 100;

                // Install path: %APPDATA%\SpaceThumbnails\deps
                var appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
                var depsDir = Path.Combine(appData, "SpaceThumbnails", "deps");
                Directory.CreateDirectory(depsDir);

                var startInfo = new ProcessStartInfo
                {
                    FileName = tempFile,
                    // /DESTINATION must be the parent folder if we want it to create LibreOfficePortable folder?
                    // No, PortableApps installers use DESTINATION as the *target* directory.
                    // If we want it in ...\deps\LibreOfficePortable, we should specify that?
                    // Usually they extract *into* the destination.
                    // But if I specify ...\deps, it might extract to ...\deps\LibreOfficePortable or just ...\deps\App...
                    // PortableApps behavior: If I say /DESTINATION=C:\Foo, it usually puts the app in C:\Foo\AppNamePortable or C:\Foo.
                    // Let's specify the full path including "LibreOfficePortable" to be safe.
                    Arguments = $"/DESTINATION=\"{Path.Combine(depsDir, "LibreOfficePortable")}\" /VERYSILENT /SUPPRESSMSGBOXES",
                    UseShellExecute = false,
                    CreateNoWindow = true
                };

                // Note: /VERYSILENT is standard Inno Setup / NSIS. PortableApps usually supports it.
                // If it fails to be silent, user will see the UI, which is acceptable but not ideal.
                
                var process = Process.Start(startInfo);
                if (process != null)
                {
                    await process.WaitForExitAsync();
                }

                // Cleanup
                try { File.Delete(tempFile); } catch { }

                CheckLibreOffice();
            }
            catch (Exception ex)
            {
                DownloadStatusText = "Error: " + ex.Message;
                // Wait a bit so user can read error
                await Task.Delay(5000);
                
                DownloadButtonVisibility = Visibility.Visible;
                DownloadProgressVisibility = Visibility.Collapsed;
            }
        }
    }
}
