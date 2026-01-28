import sys
import os
from pxr import Usd, UsdGeom, Gf, Vt, UsdShade

def convert_usdz_to_obj(usdz_path, obj_path):
    print(f"Opening stage: {usdz_path}")
    stage = Usd.Stage.Open(usdz_path)
    if not stage:
        print(f"Error: Could not open {usdz_path}")
        sys.exit(1)

    time = Usd.TimeCode.Default()
    xform_cache = UsdGeom.XformCache(time)
    
    # Prepare MTL file path
    mtl_path = os.path.splitext(obj_path)[0] + ".mtl"
    mtl_filename = os.path.basename(mtl_path)
    
    materials = {} # Store material name -> (r, g, b)

    with open(obj_path, 'w') as f, open(mtl_path, 'w') as f_mtl:
        f.write(f"# Converted from USDZ by space-thumbnails\n")
        f.write(f"mtllib {mtl_filename}\n")
        
        f_mtl.write(f"# Materials for {os.path.basename(obj_path)}\n")
        
        vertex_offset = 1
        uv_offset = 1
        normal_offset = 1
        
        # Traverse all prims
        for prim in stage.Traverse():
            if prim.IsA(UsdGeom.Mesh):
                mesh = UsdGeom.Mesh(prim)
                
                # Get mesh data
                points = mesh.GetPointsAttr().Get(time)
                if not points:
                    continue
                
                face_vertex_counts = mesh.GetFaceVertexCountsAttr().Get(time)
                face_vertex_indices = mesh.GetFaceVertexIndicesAttr().Get(time)
                
                if not face_vertex_counts or not face_vertex_indices:
                    continue

                # Get World Transform
                transform = xform_cache.GetLocalToWorldTransform(prim)
                
                # --- Vertices ---
                f.write(f"o {prim.GetName()}\n")
                for p in points:
                    tp = transform.Transform(p)
                    f.write(f"v {tp[0]} {tp[1]} {tp[2]}\n")
                
                # --- Normals ---
                normals = mesh.GetNormalsAttr().Get(time)
                has_normals = False
                if normals and len(normals) == len(points):
                    has_normals = True
                    # Normals need rotation transform only (no translation)
                    # Use TransformDir on the Matrix4d to rotate vectors
                    for n in normals:
                        tn = transform.TransformDir(n)
                        f.write(f"vn {tn[0]} {tn[1]} {tn[2]}\n")
                elif normals and len(normals) == len(face_vertex_indices):
                    # Face-varying normals (not supported easily in this simple writer, skipping)
                    pass

                # --- UVs (st) ---
                # Try to find "st" or "uv" primvar
                uvs = None
                primvars_api = UsdGeom.PrimvarsAPI(mesh)
                pv_st = primvars_api.GetPrimvar("st")
                if not pv_st:
                    pv_st = primvars_api.GetPrimvar("uv")
                
                has_uvs = False
                if pv_st:
                    uv_data = pv_st.Get(time)
                    if uv_data:
                        # Check interpolation. For OBJ we usually need vertex or face-varying.
                        # If constant/uniform, ignore.
                        interp = pv_st.GetInterpolation()
                        if interp in [UsdGeom.Tokens.vertex, UsdGeom.Tokens.varying, UsdGeom.Tokens.faceVarying]:
                             # If indices exist, we need to handle them. 
                             # For simplicity, if indexed, flatten? 
                             # Or just write all UVs and index them?
                             # OBJ 'vt' is just a list.
                             # If faceVarying (common for UVs), they match face_vertex_indices count usually.
                             # If vertex, they match points count.
                             
                             # Let's write them all out.
                             if pv_st.IsIndexed():
                                 # Flattening is safer for simple OBJ writer
                                 # But we can just write the raw values and use indices if we are careful.
                                 # Let's just flatten to be safe if indexed? 
                                 # Actually, `Get` usually returns the values? No, `Get` returns values. `GetIndices` returns indices.
                                 # If indexed, `Get` might return the palette?
                                 # Documentation says Get returns the value. 
                                 # If indexed, we might need to resolve.
                                 # But pv_st.ComputeFlattened() is best.
                                 uv_data = pv_st.ComputeFlattened(time)
                             
                             if uv_data:
                                 has_uvs = True
                                 for uv in uv_data:
                                     f.write(f"vt {uv[0]} {1.0 - uv[1]}\n") # Flip V for OBJ usually
                
                # --- Material / Color ---
                # Check DisplayColor
                display_color_attr = mesh.GetDisplayColorAttr()
                display_color = display_color_attr.Get(time)
                
                mat_name = "default"
                if display_color:
                    # Take the first color (assuming uniform for the mesh)
                    c = display_color[0]
                    mat_name = f"mat_{prim.GetName()}"
                    # Sanitize name
                    mat_name = "".join(c for c in mat_name if c.isalnum() or c in ('_','-'))
                    
                    if mat_name not in materials:
                        materials[mat_name] = c
                        f_mtl.write(f"newmtl {mat_name}\n")
                        f_mtl.write(f"Kd {c[0]} {c[1]} {c[2]}\n")
                        f_mtl.write("d 1.0\n")
                        f_mtl.write("illum 2\n\n")
                    
                    f.write(f"usemtl {mat_name}\n")

                # --- Faces ---
                idx_ptr = 0
                # UV index pointer
                uv_idx_ptr = 0
                
                for count in face_vertex_counts:
                    f.write("f")
                    for i in range(count):
                        # OBJ indices are 1-based
                        
                        # Vertex Index
                        v_idx = face_vertex_indices[idx_ptr + i] + vertex_offset
                        
                        # UV Index
                        vt_idx = ""
                        if has_uvs:
                            # If vertex interpolation
                            if len(uv_data) == len(points):
                                vt_idx = str(face_vertex_indices[idx_ptr + i] + uv_offset)
                            # If faceVarying interpolation (matches indices count)
                            elif len(uv_data) == len(face_vertex_indices):
                                vt_idx = str(idx_ptr + i + uv_offset)
                        
                        # Normal Index
                        vn_idx = ""
                        if has_normals:
                            vn_idx = str(face_vertex_indices[idx_ptr + i] + normal_offset)
                            
                        # Format: v/vt/vn or v//vn or v/vt
                        if has_normals:
                            f.write(f" {v_idx}/{vt_idx}/{vn_idx}")
                        elif has_uvs:
                             f.write(f" {v_idx}/{vt_idx}")
                        else:
                             f.write(f" {v_idx}")
                             
                    f.write("\n")
                    idx_ptr += count
                
                vertex_offset += len(points)
                if has_uvs:
                    uv_offset += len(uv_data)
                if has_normals:
                    normal_offset += len(normals)

    print(f"Exported to {obj_path} and {mtl_path}")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python usdz2obj.py input.usdz output.obj")
        sys.exit(1)
    
    convert_usdz_to_obj(sys.argv[1], sys.argv[2])
