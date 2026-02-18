
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public class ShellThumbnail
{
    [ComImport]
    [Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IShellItemImageFactory
    {
        void GetImage(
            [In] SIZE size,
            [In] int flags,
            [Out] out IntPtr phbm);
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct SIZE
    {
        public int cx;
        public int cy;
        public SIZE(int x, int y) { cx = x; cy = y; }
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
    public static extern void SHCreateItemFromParsingName(
        [MarshalAs(UnmanagedType.LPWStr)] string pszPath,
        IntPtr pbc,
        [MarshalAs(UnmanagedType.LPStruct)] Guid riid,
        [MarshalAs(UnmanagedType.Interface)] out IShellItemImageFactory ppv);
        
    public static void GetThumbnail(string path)
    {
        try {
            Console.WriteLine("Getting IShellItemImageFactory for: " + path);
            IShellItemImageFactory factory;
            Guid IID_IShellItemImageFactory = new Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b");
            SHCreateItemFromParsingName(path, IntPtr.Zero, IID_IShellItemImageFactory, out factory);
            
            Console.WriteLine("Got factory. Requesting thumbnail...");
            IntPtr hbitmap;
            // SIIGBF_THUMBNAILONLY = 0x00 (default) or 0x08? 
            // SIIGBF_RESIZETOFIT = 0x00
            // SIIGBF_BIGGERSIZEOK = 0x01
            // SIIGBF_MEMORYONLY = 0x02
            // SIIGBF_ICONONLY = 0x04
            // SIIGBF_THUMBNAILONLY = 0x08
            // SIIGBF_INCACHEONLY = 0x10
            
            // Try explicit thumbnail request (0x08) to force provider call
            // Also, pass 0x04 (SIIGBF_ICONONLY) if you want icon.
            // But we want to test if the provider works.
            
            Console.WriteLine("Requesting THUMBNAILONLY (0x08)...");
            IntPtr hbitmap;
            // 0x08 = SIIGBF_THUMBNAILONLY
            ((IShellItemImageFactory)factory).GetImage(new SIZE(256, 256), 0x08, out hbitmap); 
            
            Console.WriteLine("SUCCESS: Got thumbnail handle: " + hbitmap);
        }
        catch (Exception ex)
        {
            Console.WriteLine("ERROR: " + ex.Message);
            Console.WriteLine("HRESULT: 0x" + ex.HResult.ToString("X"));
        }
    }
}
"@

$path = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\assets\test.step"
if (!(Test-Path $path)) {
    Write-Host "Error: Test file not found at $path"
    exit
}

[ShellThumbnail]::GetThumbnail($path)
