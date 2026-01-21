using Microsoft.Win32;
using System;
using System.Diagnostics;
using System.IO;

namespace SpaceThumbnails.ControlPanel
{
    public static class RegistryHelper
    {
        [System.Runtime.InteropServices.DllImport("shell32.dll")]
        private static extern void SHChangeNotify(int wEventId, int uFlags, IntPtr dwItem1, IntPtr dwItem2);

        private const int SHCNE_ASSOCCHANGED = 0x08000000;
        private const int SHCNF_IDLIST = 0x0000;

        public static void UpdateItemStatus(FormatItem item)
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

        public static void CleanRegistration(string relativePath, string subKey, string targetGuid)
        {
            string[] roots = { 
                "HKEY_CURRENT_USER\\Software\\Classes", 
                "HKEY_LOCAL_MACHINE\\SOFTWARE\\Classes" 
            };
            
            foreach (var root in roots)
            {
                string fullKey = $"{root}\\{relativePath}\\{subKey}";
                try 
                {
                    string val = Registry.GetValue(fullKey, "", null) as string;
                    if (string.Equals(val, targetGuid, StringComparison.OrdinalIgnoreCase))
                    {
                        RunRegCommand("delete", fullKey, "/f");
                    }
                }
                catch { }
            }
        }

        public static void RunRegCommand(string operation, string key, string args)
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
                    SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, IntPtr.Zero, IntPtr.Zero);
                }
            }
            catch (Exception)
            {
                // Log or handle error
            }
        }

        public static void RegisterDll(string dllPath)
        {
             if (!File.Exists(dllPath))
                throw new FileNotFoundException($"DLL not found at {dllPath}");

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
            }
        }

        public static bool IsExtensionRegistered(string extension, string guid)
        {
            return CheckRegistration(extension, guid);
        }

        public static bool RegisterExtension(string extension, string guid)
        {
            string key = $"HKEY_CURRENT_USER\\Software\\Classes\\{extension}\\shellex\\{{e357fccd-a995-4576-b01f-234630154e96}}";
            RunRegCommand("add", key, $"/ve /d \"{guid}\" /f");
            return IsExtensionRegistered(extension, guid);
        }

        public static bool UnregisterExtension(string extension, string guid)
        {
            // We only unregister if it matches OUR guid, to avoid breaking other apps
            if (!IsExtensionRegistered(extension, guid)) return true;

            string key = $"HKEY_CURRENT_USER\\Software\\Classes\\{extension}\\shellex\\{{e357fccd-a995-4576-b01f-234630154e96}}";
            RunRegCommand("delete", key, "/f");
            return !IsExtensionRegistered(extension, guid);
        }

        private static bool CheckRegistration(string extension, string targetGuid)
        {
            try
            {
                string thumbnailProviderKey = "\\shellex\\{e357fccd-a995-4576-b01f-234630154e96}";
                string extVal = Registry.GetValue($"HKEY_CLASSES_ROOT\\{extension}{thumbnailProviderKey}", "", null) as string;
                if (string.Equals(extVal, targetGuid, StringComparison.OrdinalIgnoreCase)) return true;

                extVal = Registry.GetValue($"HKEY_CURRENT_USER\\Software\\Classes\\{extension}{thumbnailProviderKey}", "", null) as string;
                if (string.Equals(extVal, targetGuid, StringComparison.OrdinalIgnoreCase)) return true;
                
                return false;
            }
            catch
            {
                return false;
            }
        }
    }
}
