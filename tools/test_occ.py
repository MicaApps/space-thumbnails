
import sys
import os

print("Importing OCP...", flush=True)
try:
    from OCP.STEPControl import STEPControl_Reader
    from OCP.STEPCAFControl import STEPCAFControl_Reader
    from OCP.TDocStd import TDocStd_Document
    from OCP.TCollection import TCollection_ExtendedString
    from OCP.IFSelect import IFSelect_RetDone
    print("Imports successful.", flush=True)
except ImportError as e:
    print(f"Import failed: {e}", flush=True)
    sys.exit(1)

def test_read(path):
    print(f"Testing read on {path}...", flush=True)
    if not os.path.exists(path):
        print("File does not exist.", flush=True)
        return

    try:
        reader = STEPCAFControl_Reader()
        reader.SetColorMode(True)
        reader.SetNameMode(True)
        
        status = reader.ReadFile(path)
        print(f"ReadFile status: {status}", flush=True)
        
        if status == IFSelect_RetDone:
            print("Status is RetDone (OK).", flush=True)
            
            # Try using XCAFApp_Application
            from OCP.XCAFApp import XCAFApp_Application
            app = XCAFApp_Application.GetApplication_s()
            doc = TDocStd_Document(TCollection_ExtendedString("MDTV-XCAF"))
            app.NewDocument(TCollection_ExtendedString("MDTV-XCAF"), doc)
            
            print("Document created via App.", flush=True)
            
            ok = reader.Transfer(doc)
            print(f"Transfer result: {ok}", flush=True)
        else:
            print("Read failed.", flush=True)
            
    except Exception as e:
        print(f"Exception during read: {e}", flush=True)

if __name__ == "__main__":
    if len(sys.argv) > 1:
        test_read(sys.argv[1])
    else:
        print("No input file provided.", flush=True)
