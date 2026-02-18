
$uniqueId = [Guid]::NewGuid().ToString("N")
$className = "ShellThumbnail_$uniqueId"

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public class $className
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
        
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern IntPtr LoadLibrary(string lpFileName);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool FreeLibrary(IntPtr hModule);

    public static void GetThumbnail(string path)
    {
        try {
            Console.WriteLine("Current ApartmentState: " + System.Threading.Thread.CurrentThread.GetApartmentState());
            
            string dllPath = Environment.GetEnvironmentVariable("TEMP") + "\\space_thumbnails.dll";
            Console.WriteLine("Attempting to LoadLibrary: " + dllPath);
            IntPtr hModule = LoadLibrary(dllPath);
            if (hModule == IntPtr.Zero)
            {
                Console.WriteLine("ERROR: LoadLibrary failed. Error: " + Marshal.GetLastWin32Error());
                return;
            }
            Console.WriteLine("SUCCESS: LoadLibrary loaded module at: " + hModule);
            FreeLibrary(hModule);

            Console.WriteLine("Getting IShellItemImageFactory for: " + path);
            IShellItemImageFactory factory;
            Guid IID_IShellItemImageFactory = new Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b");
            SHCreateItemFromParsingName(path, IntPtr.Zero, IID_IShellItemImageFactory, out factory);
            
            Console.WriteLine("Got factory. Requesting THUMBNAILONLY (0x08)...");
            IntPtr hbitmap;
            // 0x08 = SIIGBF_THUMBNAILONLY
            // 0x00 = SIIGBF_RESIZETOFIT (default, allows icon fallback)
            
            // We use 0x08 to FORCE the thumbnail provider. If no provider, it fails.
            factory.GetImage(new SIZE(256, 256), 0x08, out hbitmap); 
            
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

Invoke-Expression "[$className]::GetThumbnail('$path')"
