import sys
import os
import FreeCAD
import Part
import Mesh
import MeshPart

def log_debug(msg):
    try:
        with open(r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\st_debug.log", "a", encoding="utf-8") as f:
            f.write(f"[Python] {msg}\n")
    except:
        pass

def get_script_args():
    # Hack to bypass FreeCADCmd's own argument parsing
    script_name = "step2obj.py"
    args = []
    found_script = False
    
    log_debug(f"DEBUG: sys.argv = {sys.argv}")

    for arg in sys.argv:
        if found_script:
            if arg == "--":
                continue
            args.append(arg)
        elif script_name in arg:
            found_script = True
    return args

def get_paths():
    # Try getting paths from environment variables first (safest way to bypass FreeCAD arg parsing)
    in_env = os.environ.get("STEP2OBJ_INPUT")
    out_env = os.environ.get("STEP2OBJ_OUTPUT")
    
    if in_env and out_env:
        return in_env, out_env

    # Fallback to sys.argv parsing (legacy/manual run)
    args = get_script_args()
    if len(args) >= 2:
        return args[0], args[1]
        
    return None, None

def main():
    try:
        log_debug("Starting script...")
        in_path, out_path = get_paths()

        if not in_path or not out_path:
            log_debug("Error: Env vars missing")
            return

        in_path = in_path.strip('"')
        out_path = out_path.strip('"')
        
        # Absolute paths are safer
        in_path = os.path.abspath(in_path)
        out_path = os.path.abspath(out_path)

        log_debug(f"Input: {in_path}")
        log_debug(f"Output: {out_path}")

        if not os.path.exists(in_path):
            log_debug(f"Error: Input file does not exist: {in_path}")
            return

        doc = FreeCAD.newDocument()
        log_debug("Importing STEP...")
        Part.insert(in_path, doc.Name)

        meshes = []
        log_debug("Tessellating...")
        for obj in doc.Objects:
            if hasattr(obj, "Shape"):
                try:
                    # Tesselation parameters
                    # LinearDeflection 1.0 is very coarse but much faster.
                    # For thumbnails (256x256), 1mm deviation is barely visible.
                    mesh = MeshPart.meshFromShape(
                        Shape=obj.Shape,
                        LinearDeflection=1.0,
                        AngularDeflection=0.523599,
                        Relative=False
                    )
                    meshes.append(mesh)
                except Exception as e:
                    log_debug(f"Warning: Failed to mesh object {obj.Name}: {e}")

        if not meshes:
            log_debug("Error: No meshes generated")
            return

        log_debug(f"Merging {len(meshes)} meshes...")
        base = meshes[0]
        for m in meshes[1:]:
            base.addMesh(m)
            
        # Decimate mesh to reduce file size and loading time
        log_debug(f"Original facets: {base.CountFacets}")
        try:
            # Decimate with target size
            # Error message said: decimate(targetSize=int)
            base.decimate(targetSize=50000)
            log_debug(f"Decimated facets: {base.CountFacets}")
        except Exception as e:
            log_debug(f"Decimation warning: {e}")

        log_debug(f"Exporting to {out_path}...")
        
        # Write directly from the Mesh object
        base.write(out_path)
        
        if os.path.exists(out_path):
             log_debug("Export verified: File exists.")
        else:
             log_debug("Export FAILED: File does not exist after write.")
        
        log_debug("Done.")

    except Exception as e:
        log_debug(f"FATAL ERROR: {e}")
        import traceback
        traceback.print_exc()

# if __name__ == "__main__":
#    main()

# In FreeCADCmd, the script is imported, so __name__ is the filename.
# We just want to run main() directly.
main()
