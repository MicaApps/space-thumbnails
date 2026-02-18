$code = @"
using System;
using System.Runtime.InteropServices;

public class ShellNotify {
    [DllImport("shell32.dll")]
    public static extern void SHChangeNotify(uint wEventId, uint uFlags, IntPtr dwItem1, IntPtr dwItem2);

    public const uint SHCNE_UPDATEITEM = 0x00002000;
    public const uint SHCNF_PATHW = 0x0005;

    public static void Refresh(string path) {
        IntPtr ptr = Marshal.StringToHGlobalUni(path);
        SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, ptr, IntPtr.Zero);
        Marshal.FreeHGlobal(ptr);
    }
}
"@

Add-Type -TypeDefinition $code
$path = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\assets\test.step"
[ShellNotify]::Refresh($path)
Write-Host "Sent update notification for: $path"
