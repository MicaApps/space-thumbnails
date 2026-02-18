
$code = @"
using System;
using System.Runtime.InteropServices;

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
    void Initialize(object pstream, uint grfMode);
}

public class Tester
{
    public static void Test(string clsidStr)
    {
        Console.WriteLine("Testing CLSID: " + clsidStr);
        Guid clsid = new Guid(clsidStr);
        Type type = Type.GetTypeFromCLSID(clsid);
        if (type == null) {
            Console.WriteLine("Type not found in registry.");
            return;
        }
        
        try {
            object instance = Activator.CreateInstance(type);
            Console.WriteLine("Object created successfully.");
            
            IInitializeWithFile initFile = instance as IInitializeWithFile;
            if (initFile != null) {
                Console.WriteLine("SUCCESS: Object supports IInitializeWithFile");
            } else {
                Console.WriteLine("FAILURE: Object does NOT support IInitializeWithFile");
            }

            IInitializeWithStream initStream = instance as IInitializeWithStream;
            if (initStream != null) {
                Console.WriteLine("SUCCESS: Object supports IInitializeWithStream");
            } else {
                Console.WriteLine("INFO: Object does NOT support IInitializeWithStream (Expected if removed)");
            }

        } catch (Exception e) {
            Console.WriteLine("Error creating instance: " + e.Message);
        }
    }
}
"@

Add-Type -TypeDefinition $code
[Tester]::Test("{662657D4-0325-4632-9154-116584281360}")
