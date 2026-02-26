
$code = @"
using System;
using System.Runtime.InteropServices;
public class Shell {
    [DllImport("shell32.dll")]
    public static extern void SHChangeNotify(long wEventId, uint uFlags, IntPtr dwItem1, IntPtr dwItem2);
}
"@
Add-Type -TypeDefinition $code
# SHCNE_ASSOCCHANGED = 0x08000000, SHCNF_IDLIST = 0x0000
# This forces a full refresh of the icon cache and file associations
[Shell]::SHChangeNotify(0x08000000, 0x0000, [IntPtr]::Zero, [IntPtr]::Zero)
Write-Host "Refreshed Icon Cache and File Associations." -ForegroundColor Green
