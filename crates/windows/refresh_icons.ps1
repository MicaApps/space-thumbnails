$code = '[DllImport("shell32.dll")] public static extern void SHChangeNotify(int wEventId, int uFlags, IntPtr dwItem1, IntPtr dwItem2);'
$type = Add-Type -MemberDefinition $code -Name "Win32ChangeNotify" -Namespace Win32 -PassThru
$type::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)
Write-Host "SHChangeNotify called."

ie4uinit.exe -show
Write-Host "ie4uinit called."

Stop-Process -Name explorer -Force
Start-Process explorer
Write-Host "Explorer restarted."
