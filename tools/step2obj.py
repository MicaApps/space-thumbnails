import sys
import os
import FreeCAD
import Part
import Mesh
import MeshPart

def get_script_args():
    # Hack to bypass FreeCADCmd's own argument parsing
    script_name = "step2obj.py"
    args = []
    found_script = False
    
    print(f"DEBUG: sys.argv = {sys.argv}")

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
        in_path, out_path = get_paths()

        if not in_path or not out_path:
            print("Usage: Set STEP2OBJ_INPUT and STEP2OBJ_OUTPUT env vars")
            print("       OR: step2obj.py input.step output.obj")
            print(f"Debug args: {sys.argv}")
            return

        in_path = in_path.strip('"')
        out_path = out_path.strip('"')
        
        # Absolute paths are safer
        in_path = os.path.abspath(in_path)
        out_path = os.path.abspath(out_path)

        print(f"Input: {in_path}")
        print(f"Output: {out_path}")

        if not os.path.exists(in_path):
            print(f"Error: Input file does not exist: {in_path}")
            return

        doc = FreeCAD.newDocument()
        print("Importing STEP...")
        Part.insert(in_path, doc.Name)

        meshes = []
        print("Tessellating...")
        for obj in doc.Objects:
            if hasattr(obj, "Shape"):
                try:
                    # Tesselation parameters
                    mesh = MeshPart.meshFromShape(
                        Shape=obj.Shape,
                        LinearDeflection=0.1,
                        AngularDeflection=0.523599,
                        Relative=False
                    )
                    meshes.append(mesh)
                except Exception as e:
                    print(f"Warning: Failed to mesh object {obj.Name}: {e}")

        if not meshes:
            print("Error: No meshes generated")
            return

        print(f"Merging {len(meshes)} meshes...")
        base = meshes[0]
        for m in meshes[1:]:
            base.addMesh(m)

        print(f"Exporting to {out_path}...")
        # FreeCAD Mesh module export
        # Mesh.export([base], out_path) # This expects Document Objects
        
        # Write directly from the Mesh object
        base.write(out_path)
        
        print("Done.")

    except Exception as e:
        print(f"FATAL ERROR: {e}")
        import traceback
        traceback.print_exc()

# if __name__ == "__main__":
#    main()

# In FreeCADCmd, the script is imported, so __name__ is the filename.
# We just want to run main() directly.
main()
