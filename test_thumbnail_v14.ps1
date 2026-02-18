$code = @"
using System;
using System.Runtime.InteropServices;

namespace TestV14 {

    [ComImport]
    [Guid("b7d14566-0509-4cce-a71f-0a554233bd9b")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IInitializeWithFile
    {
        void Initialize([MarshalAs(UnmanagedType.LPWStr)] string pszFilePath, uint grfMode);
    }

    [ComImport]
    [Guid("e357fccd-a995-4576-b01f-234630154e96")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IThumbnailProvider
    {
        void GetThumbnail(uint cx, out IntPtr phbmp, out uint pdwAlpha);
    }

    public class Tester
    {
        public static void Test(string clsidStr, string filePath)
        {
            Console.WriteLine("Testing CLSID: " + clsidStr);
            Console.WriteLine("File path: " + filePath);
            
            try {
                Guid clsid = new Guid(clsidStr);
                Type type = Type.GetTypeFromCLSID(clsid);
                if (type == null) {
                    Console.WriteLine("Type not found.");
                    return;
                }

                object instance = Activator.CreateInstance(type);
                Console.WriteLine("Object created.");
                
                IInitializeWithFile initFile = instance as IInitializeWithFile;
                if (initFile != null) {
                    Console.WriteLine("Calling Initialize (File)...");
                    initFile.Initialize(filePath, 0);
                    Console.WriteLine("Initialize called successfully.");
                } else {
                    Console.WriteLine("FAILURE: IInitializeWithFile not supported.");
                    return;
                }

                IThumbnailProvider thumb = instance as IThumbnailProvider;
                if (thumb != null) {
                    Console.WriteLine("Calling GetThumbnail...");
                    IntPtr hbmp;
                    uint alpha;
                    thumb.GetThumbnail(256, out hbmp, out alpha);
                    Console.WriteLine("GetThumbnail returned handle: " + hbmp);
                } else {
                     Console.WriteLine("IThumbnailProvider not supported.");
                }
            } catch (Exception e) {
                Console.WriteLine("Error: " + e.ToString());
            }
        }
    }
}
"@

Add-Type -TypeDefinition $code
$path = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\assets\test.step"
[TestV14.Tester]::Test("{662657D4-0325-4632-9154-116584281360}", $path)
