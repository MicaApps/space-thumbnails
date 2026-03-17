$sampleDir = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\control-panel\Samples"
$docxFiles = Get-ChildItem -Path $sampleDir -Filter "*.docx"
if ($docxFiles.Count -eq 0) {
    Write-Error "No .docx files found in $sampleDir"
    exit 1
}

$path = $docxFiles[0].FullName
Write-Host "Using file: $path"

$baseOutputPath = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\crates\windows\shell_via_ps_v4"

# 加载必要的程序集
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$code = @"
using System;
using System.Runtime.InteropServices;
using System.Drawing;
using System.IO;

public class ShellThumbTestV4 {
    [StructLayout(LayoutKind.Sequential)]
    public struct SIZE { public int cx; public int cy; }

    [ComImport, Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IShellItem {
        void BindToHandler(IntPtr pbc, [In] ref Guid bhid, [In] ref Guid riid, [MarshalAs(UnmanagedType.Interface)] out object ppv);
        void GetParent(out IShellItem ppsi);
        void GetDisplayName(uint sigdnName, out IntPtr ppszName);
        void GetAttributes(uint sfgaoMask, out uint psfgaoAttribs);
        void Compare(IShellItem psi, uint hint, out int piOrder);
    }

    [ComImport, Guid("bcc18b79-ba16-445f-999d-394c136fd031"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IShellItemImageFactory {
        [PreserveSig]
        int GetImage(SIZE size, int flags, out IntPtr hBitmap);
    }

    [ComImport, Guid("e357fccd-a995-4576-b01f-234630154e96"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IThumbnailProvider {
        [PreserveSig]
        int GetThumbnail(int cx, out IntPtr phbmp, out int pdwAlpha);
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = true)]
    public static extern int SHCreateItemFromParsingName([MarshalAs(UnmanagedType.LPWStr)] string pszPath, IntPtr pbc, ref Guid riid, [MarshalAs(UnmanagedType.Interface)] out object ppv);

    public static Bitmap GetThumbnail(string path, int width, int height, int flags) {
        Guid IID_IShellItem = new Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe");
        Guid IID_IShellItemImageFactory = new Guid("bcc18b79-ba16-445f-999d-394c136fd031");
        Guid BHID_ThumbnailHandler = new Guid("7b010c2d-3275-4520-94e6-d99616d7a46d");
        Guid IID_IThumbnailProvider = new Guid("e357fccd-a995-4576-b01f-234630154e96");

        object shellItemObj;
        try {
            int hr = SHCreateItemFromParsingName(path, IntPtr.Zero, ref IID_IShellItem, out shellItemObj);
            if (hr != 0) {
                Console.WriteLine("SHCreateItemFromParsingName(IShellItem) failed with HRESULT: 0x{0:X}", (uint)hr);
                return null;
            }

            IShellItem shellItem = (IShellItem)shellItemObj;

            // Try 1: IShellItemImageFactory
            IShellItemImageFactory factory = shellItem as IShellItemImageFactory;
            if (factory != null) {
                IntPtr hBitmap;
                hr = factory.GetImage(new SIZE { cx = width, cy = height }, flags, out hBitmap);
                if (hr == 0 && hBitmap != IntPtr.Zero) {
                    return Image.FromHbitmap(hBitmap);
                }
                Console.WriteLine("IShellItemImageFactory.GetImage failed with HRESULT: 0x{0:X}", (uint)hr);
            }

            // Try 2: IThumbnailProvider via BindToHandler
            object providerObj;
            try {
                shellItem.BindToHandler(IntPtr.Zero, ref BHID_ThumbnailHandler, ref IID_IThumbnailProvider, out providerObj);
                IThumbnailProvider provider = providerObj as IThumbnailProvider;
                if (provider != null) {
                    IntPtr phbmp;
                    int alpha;
                    hr = provider.GetThumbnail(width, out phbmp, out alpha);
                    if (hr == 0 && phbmp != IntPtr.Zero) {
                        return Image.FromHbitmap(phbmp);
                    }
                    Console.WriteLine("IThumbnailProvider.GetThumbnail failed with HRESULT: 0x{0:X}", (uint)hr);
                }
            } catch (Exception ex) {
                Console.WriteLine("BindToHandler(IThumbnailProvider) failed: " + ex.Message);
            }

        } catch (Exception ex) {
            Console.WriteLine("Exception in GetThumbnail: " + ex.ToString());
        }
        return null;
    }
}
"@

Add-Type -TypeDefinition $code -ReferencedAssemblies System.Drawing

$flagTests = @(
    @{ Name = "RESIZETOFIT"; Value = 0x0 },
    @{ Name = "THUMBNAILONLY"; Value = 0x8 }
)

foreach ($test in $flagTests) {
    Write-Host "Testing $($test.Name) (0x$($test.Value.ToString("X")))..."
    try {
        $bmp = [ShellThumbTestV4]::GetThumbnail($path, 1024, 1024, $test.Value)
        if ($bmp) {
            $out = "$baseOutputPath`_$($test.Name).png"
            $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
            Write-Host "  Success! Saved to $out"
            $bmp.Dispose()
        } else {
            Write-Host "  Failed."
        }
    } catch {
        Write-Host "  Error: $($_.Exception.Message)"
    }
}
