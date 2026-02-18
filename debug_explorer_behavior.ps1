$dllPath = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\target_temp_debug_v10\release\space_thumbnails_windows.dll"
$filePath = "D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails6\assets\LGQGJ-00-00 轮毂抛光机总装.STEP"

$code = @'
using System;
using System.Runtime.InteropServices;
using System.Text;

namespace DebugCOM
{
    [ComImport]
    [Guid("E357FCCD-A995-4576-B01F-234630154E96")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IThumbnailProvider
    {
        void GetThumbnail(uint cx, out IntPtr phbmp, out uint pdwAlpha);
    }

    [ComImport]
    [Guid("b7d14566-0509-4cce-a71f-0a554233bd9b")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IInitializeWithFile
    {
        void Initialize([MarshalAs(UnmanagedType.LPWStr)] string pszFilePath, uint grfMode);
    }

    [ComImport]
    [Guid("b824b49d-22ac-4161-ac8a-9916e8fa3f7f")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IInitializeWithStream
    {
        void Initialize(IntPtr pstream, uint grfMode);
    }

    [ComImport]
    [Guid("00000001-0000-0000-C000-000000000046")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IClassFactory
    {
        void CreateInstance([MarshalAs(UnmanagedType.Interface)] object pUnkOuter, ref Guid riid, out IntPtr ppvObject);
        void LockServer(bool fLock);
    }

    public static class Tester
    {
        [DllImport("kernel32.dll")]
        public static extern IntPtr LoadLibrary(string dllToLoad);

        [DllImport("kernel32.dll")]
        public static extern IntPtr GetProcAddress(IntPtr hModule, string procedureName);

        [UnmanagedFunctionPointer(CallingConvention.StdCall)]
        public delegate int DllGetClassObject(ref Guid clsid, ref Guid iid, out IntPtr ppv);

        public static void Run(string dllPath, string filePath)
        {
            Console.WriteLine("Testing with DLL: " + dllPath);
            Console.WriteLine("Testing with file: " + filePath);
            
            IntPtr hModule = LoadLibrary(dllPath);
            if (hModule == IntPtr.Zero)
            {
                Console.WriteLine("LoadLibrary failed");
                return;
            }
            Console.WriteLine("LoadLibrary success");

            IntPtr pDllGetClassObject = GetProcAddress(hModule, "DllGetClassObject");
            if (pDllGetClassObject == IntPtr.Zero)
            {
                Console.WriteLine("GetProcAddress failed");
                return;
            }

            DllGetClassObject getClassObject = (DllGetClassObject)Marshal.GetDelegateForFunctionPointer(pDllGetClassObject, typeof(DllGetClassObject));

            Guid clsid = new Guid("662657D4-0325-4632-9154-116584281360");
            Guid iidClassFactory = new Guid("00000001-0000-0000-C000-000000000046");
            
            IntPtr pClassFactoryPtr;
            int hr = getClassObject(ref clsid, ref iidClassFactory, out pClassFactoryPtr);
            
            if (hr != 0)
            {
                Console.WriteLine("DllGetClassObject failed: " + hr.ToString("X"));
                return;
            }
            Console.WriteLine("Got IClassFactory ptr");

            object factoryObj = Marshal.GetObjectForIUnknown(pClassFactoryPtr);
            IClassFactory factory = (IClassFactory)factoryObj;
            
            Guid iidUnknown = new Guid("00000000-0000-0000-C000-000000000046");
            IntPtr pUnknown;
            
            try 
            {
                factory.CreateInstance(null, ref iidUnknown, out pUnknown);
                Console.WriteLine("CreateInstance(IUnknown) success. Ptr: " + pUnknown);
            }
            catch (Exception e)
            {
                Console.WriteLine("CreateInstance failed: " + e.Message);
                return;
            }

            object provider = Marshal.GetObjectForIUnknown(pUnknown);
            
            Console.WriteLine("Querying IInitializeWithStream...");
            if (provider is IInitializeWithStream)
            {
                 Console.WriteLine("IInitializeWithStream Supported!");
            }
            else
            {
                 Console.WriteLine("IInitializeWithStream NOT Supported.");
            }

            Console.WriteLine("Querying IInitializeWithFile...");
            IInitializeWithFile fileInit = provider as IInitializeWithFile;
            if (fileInit != null)
            {
                 Console.WriteLine("IInitializeWithFile Supported!");
                 try {
                    fileInit.Initialize(filePath, 0);
                    Console.WriteLine("Initialize(File) success");
                 } catch (Exception ex) {
                    Console.WriteLine("Initialize(File) failed: " + ex.Message);
                 }
            }
            else
            {
                 Console.WriteLine("IInitializeWithFile NOT Supported.");
            }
            
            Console.WriteLine("Calling GetThumbnail...");
            IThumbnailProvider thumbProvider = provider as IThumbnailProvider;
            if (thumbProvider != null)
            {
                try {
                    IntPtr hbmp;
                    uint alpha;
                    thumbProvider.GetThumbnail(256, out hbmp, out alpha);
                    Console.WriteLine("GetThumbnail success. HBITMAP: " + hbmp + ", Alpha: " + alpha);
                } catch (Exception ex) {
                    Console.WriteLine("GetThumbnail failed: " + ex.Message);
                }
            }
            else
            {
                 Console.WriteLine("IThumbnailProvider NOT Supported.");
            }
        }
    }
}
'@

Add-Type -TypeDefinition $code -Language CSharp
[DebugCOM.Tester]::Run($dllPath, $filePath)
