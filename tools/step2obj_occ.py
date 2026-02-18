import sys
import os
import time
import traceback
import shutil
import uuid

# OCP imports
try:
    from OCP.STEPControl import STEPControl_Reader
    from OCP.IGESControl import IGESControl_Reader
    from OCP.IFSelect import IFSelect_RetDone, IFSelect_ReturnStatus
    from OCP.Bnd import Bnd_Box
    from OCP.BRepBndLib import BRepBndLib
    from OCP.BRepMesh import BRepMesh_IncrementalMesh
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopAbs import TopAbs_FACE, TopAbs_SHAPE, TopAbs_REVERSED
    from OCP.TopoDS import TopoDS, TopoDS_Iterator
    from OCP.BRep import BRep_Tool
    from OCP.TopLoc import TopLoc_Location
    from OCP.STEPCAFControl import STEPCAFControl_Reader
    from OCP.TDocStd import TDocStd_Document
    from OCP.TCollection import TCollection_ExtendedString, TCollection_AsciiString
    from OCP.XCAFDoc import XCAFDoc_DocumentTool, XCAFDoc_ColorTool, XCAFDoc_ShapeTool, XCAFDoc_ColorSurf, XCAFDoc_ColorGen, XCAFDoc_ColorCurv
    from OCP.Quantity import Quantity_Color
    from OCP.TDF import TDF_LabelSequence, TDF_Label, TDF_Tool
    from OCP.TDataStd import TDataStd_Name
except ImportError as e:
    print(f"[Python-OCC] Error: OCP module not found ({e}). Please install cadquery-ocp.", flush=True)
    sys.exit(1)

def log_debug(msg):
    print(f"[Python-OCC] {msg}", flush=True)

def get_color_from_label(label, color_tool, shape_tool):
    """Try to get color from a TDF_Label"""
    if label.IsNull(): return None
    
    col = Quantity_Color()
    
    # Debug: Print entry
    entry = TCollection_AsciiString()
    TDF_Tool.Entry_s(label, entry)
    
    # Try getting color using Label directly first (if supported)
    # Then try using Shape
    
    # Modes to try: Surface, Gen, Curv
    for mode in [XCAFDoc_ColorSurf, XCAFDoc_ColorGen, XCAFDoc_ColorCurv]:
        # Method A: Use Label
        try:
            if color_tool.GetColor(label, mode, col):
                # log_debug(f"Found color on label {entry.ToCString()} mode {mode}: {col.Red()}")
                return (col.Red(), col.Green(), col.Blue())
        except Exception:
            pass

        # Method B: Use Shape
        try:
            shape = shape_tool.GetShape_s(label)
            if not shape.IsNull():
                if color_tool.GetColor(shape, mode, col):
                    # log_debug(f"Found color on shape {entry.ToCString()} mode {mode}: {col.Red()}")
                    return (col.Red(), col.Green(), col.Blue())
        except Exception:
            pass
            
    return None

def dump_label(label, level=0):
    entry = TCollection_AsciiString()
    TDF_Tool.Entry_s(label, entry)
    indent = "  " * level
    # log_debug(f"{indent}Label: {entry.ToCString()}")
    
    # Check attributes
    # We can't easily check all attributes in Python without iterating, 
    # but we can check specific tools
    
    return
    
def dump_doc(doc):
    log_debug("--- Document Dump ---")
    main = doc.Main()
    # Iterate all labels
    # This is tricky in OCP without a recursive iterator helper or manually walking
    pass

def collect_colors(label, parent_color, face_map, shape_tool, color_tool, loc=None):
    if loc is None:
        loc = TopLoc_Location()
        
    col = get_color_from_label(label, color_tool, shape_tool)
    
    # Debug: Print entry
    entry = TCollection_AsciiString()
    TDF_Tool.Entry_s(label, entry)

    if col:
        # log_debug(f"Label {entry.ToCString()} has color: {col}")
        pass
    else:
        col = parent_color
    
    shape = shape_tool.GetShape_s(label)
    if shape.IsNull(): 
        # Even if not a shape, it might be a container for subshapes or components?
        pass
    else:
        # Apply accumulated location from references
        if not loc.IsIdentity():
            shape = shape.Moved(loc)

    # Check referred shape if any (Reference)
    if shape_tool.IsReference_s(label):
        ref_label = TDF_Label()
        if shape_tool.GetReferredShape_s(label, ref_label):
             ref_entry = TCollection_AsciiString()
             TDF_Tool.Entry_s(ref_label, ref_entry)
             # log_debug(f"Label {entry.ToCString()} refers to {ref_entry.ToCString()}")
             
             # Calculate new location for the prototype
             # The current shape has the location of the reference relative to its parent
             # plus the accumulated location 'loc' applied above.
             # We need to pass the total location to the prototype.
             
             # shape.Location() now contains (parent_loc * self_loc).
             # We pass this as the 'loc' for the prototype traversal.
             collect_colors(ref_label, col, face_map, shape_tool, color_tool, loc=shape.Location())

    # 1. Assign current color to all faces of this shape (Base Color)
    if not shape.IsNull() and col:
        exp = TopExp_Explorer(shape, TopAbs_FACE)
        while exp.More():
            face = TopoDS.Face(exp.Current())
            # Use hash() for reliable mapping
            h = hash(face)
            if h not in face_map: # Don't overwrite if subshape already colored? 
                # Actually, in recursion, we usually go Top-Down. 
                # If we want sub-components to override, we should respect existing?
                # But here we are at 'label'. 
                # If 'label' has a color, it overrides parent.
                # But sub-components (processed later) should override 'label'.
                # So we can write now, and children will overwrite later.
                face_map[h] = col
            else:
                 # Overwrite is fine if we are going down?
                 # Wait, components are processed AFTER.
                 face_map[h] = col
                 
            exp.Next()

    # 2. Process SubShapes (Overrides, e.g. colored faces)
    subs = TDF_LabelSequence()
    XCAFDoc_ShapeTool.GetSubShapes_s(label, subs)
    if subs.Length() > 0:
        # log_debug(f"Label {entry.ToCString()} has {subs.Length()} subshapes")
        pass
    for i in range(1, subs.Length() + 1):
        collect_colors(subs.Value(i), col, face_map, shape_tool, color_tool, loc=loc)
        
    # 3. Process Components (Assembly)
    comps = TDF_LabelSequence()
    XCAFDoc_ShapeTool.GetComponents_s(label, comps)
    if comps.Length() > 0:
        # log_debug(f"Label {entry.ToCString()} has {comps.Length()} components")
        pass
    for i in range(1, comps.Length() + 1):
        collect_colors(comps.Value(i), col, face_map, shape_tool, color_tool, loc=loc)

def convert_step_to_obj(input_path, output_path, deflection=1.0):
    start_time = time.time()
    ext = os.path.splitext(input_path)[1].lower()
    
    # Initialize XCAF Document
    doc = TDocStd_Document(TCollection_ExtendedString("xcaf"))
    shape_tool = XCAFDoc_DocumentTool.ShapeTool_s(doc.Main())
    color_tool = XCAFDoc_DocumentTool.ColorTool_s(doc.Main())
    
    shape = None
    face_color_map = {} # Map Face -> (r,g,b)
    
    # Workaround for non-ASCII paths on Windows: copy to a safe temp file
    safe_input_path = input_path
    is_temp_copy = False
    try:
        # Check if path contains non-ASCII characters
        input_path.encode('ascii')
    except UnicodeEncodeError:
        log_debug("Detected non-ASCII characters in path. Creating temporary copy for OCP...")
        try:
            temp_dir = os.environ.get("TEMP", os.getcwd())
            safe_name = f"stp_{uuid.uuid4().hex}{ext}"
            safe_input_path = os.path.join(temp_dir, safe_name)
            shutil.copy2(input_path, safe_input_path)
            is_temp_copy = True
            log_debug(f"Copied to: {safe_input_path}")
        except Exception as e:
            log_debug(f"Failed to create temp copy: {e}")
            # Fallback to original path and hope for the best
            safe_input_path = input_path

    try:
        if ext in ['.stp', '.step']:
            log_debug(f"Reading STEP (Color Mode): {input_path}")
            
            reader = STEPCAFControl_Reader()
            reader.SetColorMode(True)
            reader.SetNameMode(True)
            reader.SetLayerMode(True)
            reader.SetPropsMode(True)
            
            t_read_start = time.time()
            status = reader.ReadFile(safe_input_path)
            t_read_end = time.time()
            log_debug(f"ReadFile took {t_read_end - t_read_start:.2f}s")
            
            if status != IFSelect_RetDone:
                log_debug(f"Error: Cannot read STEP file (Status: {status}).")
                return False
                
            t_trans_start = time.time()
            if not reader.Transfer(doc):
                 log_debug("Error: Transfer to XCAF failed.")
                 # Continue anyway? No, transfer failed.
            else:
                 t_trans_end = time.time()
                 log_debug(f"Transfer successful (took {t_trans_end - t_trans_start:.2f}s).")
                
                 # Debug: Check for colors in document
                 color_labels = TDF_LabelSequence()
                 color_tool.GetColors(color_labels)
                 log_debug(f"Document contains {color_labels.Length()} color definitions.")
                
                 labels = TDF_LabelSequence()
                 shape_tool.GetFreeShapes(labels)
                
                 log_debug(f"XCAF Found {labels.Length()} free shapes.")
                
                 if labels.IsEmpty():
                     log_debug("Warning: No free shapes found, trying normal STEP reader fallback...")
                     # Fallback to normal reader if XCAF fails to find shapes
                     reader_std = STEPControl_Reader()
                     reader_std.ReadFile(safe_input_path)
                     reader_std.TransferRoots()
                     shape = reader_std.OneShape()
                 else:
                     from OCP.TopoDS import TopoDS_Compound
                     from OCP.BRep import BRep_Builder
                     builder = BRep_Builder()
                     shape = TopoDS_Compound()
                     builder.MakeCompound(shape)
                    
                     # Collect colors and build compound
                     # Check output format early to decide if we need colors
                     out_format = os.environ.get("STEP2OBJ_FORMAT", "OBJ").upper()
                     need_colors = (out_format != "STL")

                     t_col_start = time.time()
                     for i in range(1, labels.Length() + 1):
                         lab = labels.Value(i)
                         s = shape_tool.GetShape_s(lab)
                         if not s.IsNull():
                             builder.Add(shape, s)
                             if need_colors:
                                 collect_colors(lab, None, face_color_map, shape_tool, color_tool)
                     t_col_end = time.time()
                            
                     log_debug(f"Compound shape created with {labels.Length()} components.")
                     if need_colors:
                         log_debug(f"Mapped colors for {len(face_color_map)} faces in {t_col_end - t_col_start:.2f}s.")
                     else:
                         log_debug("Skipped color collection for STL export.")
        else:
            # IGES or others
            log_debug(f"Reading IGES: {input_path}")
            reader = IGESControl_Reader()
            if reader.ReadFile(safe_input_path) == IFSelect_RetDone:
                reader.TransferRoots()
                shape = reader.OneShape()

        if not shape or shape.IsNull():
            log_debug("Error: No valid geometry found.")
            return False

        t0 = time.time()
        
        # Calculate BBox for diagnostic
        bbox = Bnd_Box()
        BRepBndLib.Add_s(shape, bbox)
        xmin, ymin, zmin, xmax, ymax, zmax = bbox.Get()
        diag = ((xmax-xmin)**2 + (ymax-ymin)**2 + (zmax-zmin)**2)**0.5
        log_debug(f"BBox: [{xmin:.2f}, {ymin:.2f}, {zmin:.2f}] - [{xmax:.2f}, {ymax:.2f}, {zmax:.2f}] Diag: {diag:.2f}")

        # Mesh
        # Use environment variable for deflection if set
        env_deflection = os.environ.get("STEP2OBJ_DEFLECTION")
        deflection = 10.0 # default
        if env_deflection:
            try:
                deflection = float(env_deflection)
            except ValueError:
                pass
        
        env_ang_deflection = os.environ.get("STEP2OBJ_ANG_DEFLECTION")
        ang_deflection = 0.5 # default 0.5 rad (~28 deg)
        if env_ang_deflection:
            try:
                ang_deflection = float(env_ang_deflection)
            except ValueError:
                pass

        env_relative = os.environ.get("STEP2OBJ_RELATIVE")
        is_relative = False
        if env_relative and env_relative.lower() == "true":
            is_relative = True

        log_debug(f"Meshing (lin_deflection={deflection}, ang_deflection={ang_deflection}, relative={is_relative}, parallel=True)...")
        # BRepMesh_IncrementalMesh(shape, lin_deflection, is_relative, ang_deflection, parallel)
        BRepMesh_IncrementalMesh(shape, deflection, is_relative, ang_deflection, True)
        t1 = time.time()
        log_debug(f"Meshing took {t1-t0:.2f}s")
        
        # Count triangles
        trsf = TopLoc_Location()
        tri_count = 0
        exp = TopExp_Explorer(shape, TopAbs_FACE)
        while exp.More():
             face = TopoDS.Face(exp.Current())
             tri = BRep_Tool.Triangulation_s(face, trsf)
             if tri:
                 tri_count += tri.NbTriangles()
             exp.Next()
        log_debug(f"Total Triangles: {tri_count}")

        # Check output format
        out_format = os.environ.get("STEP2OBJ_FORMAT", "OBJ").upper()
        
        t2 = time.time()
        if out_format == "STL":
            log_debug("Exporting to STL (binary)...")
            from OCP.StlAPI import StlAPI_Writer
            writer = StlAPI_Writer()
            writer.Write(shape, output_path)
            log_debug("STL Export successful.")
            t3 = time.time()
            log_debug(f"Exporting took {t3-t2:.2f}s")
            return True
        
        # Export OBJ (default)
        log_debug("Exporting OBJ + MTL...")
        materials = {}
        mtl_path = os.path.splitext(output_path)[0] + ".mtl"
        mtl_name = os.path.basename(mtl_path)
        
        try:
            with open(output_path, 'w') as f:
                f.write(f"mtllib {mtl_name}\n")
                
                exp = TopExp_Explorer(shape, TopAbs_FACE)
                v_offset = 1
                while exp.More():
                    face = TopoDS.Face(exp.Current())
                    
                    # Try to get color from map
                    h = hash(face)
                    color = face_color_map.get(h)
                    
                    if not color:
                        # Fallback to old lookup if map fails (e.g. if hash changed after meshing - shouldn't happen for HashCode of underlying shape unless shape changed)
                        # color = find_color_for_shape(face, shape_tool, color_tool) 
                        pass
                    
                    if not color:
                        color = (0.8, 0.8, 0.8)
                    
                    ckey = tuple(round(c, 3) for c in color)
                    if ckey not in materials:
                        mname = f"mat_{len(materials)}"
                        materials[ckey] = mname
                    else:
                        mname = materials[ckey]
                    
                    loc = TopLoc_Location()
                    tri = BRep_Tool.Triangulation_s(face, loc)
                    if tri:
                        if tri.NbNodes() == 0:
                            continue
                            
                        # Write vertices (v)
                        trsf = loc.Transformation()
                        for i in range(1, tri.NbNodes() + 1):
                            p = tri.Node(i).Transformed(trsf)
                            f.write(f"v {p.X():.4f} {p.Y():.4f} {p.Z():.4f}\n")
                        
                        # Write faces (f)
                        f.write(f"usemtl {mname}\n")
                        is_reversed = face.Orientation() == TopAbs_REVERSED
                        for i in range(1, tri.NbTriangles() + 1):
                            n1, n2, n3 = tri.Triangle(i).Get()
                            if is_reversed:
                                f.write(f"f {n1+v_offset-1} {n3+v_offset-1} {n2+v_offset-1}\n")
                            else:
                                f.write(f"f {n1+v_offset-1} {n2+v_offset-1} {n3+v_offset-1}\n")
                        v_offset += tri.NbNodes()
                    exp.Next()
                    
            # Write MTL
            log_debug(f"Writing MTL to {mtl_path} with {len(materials)} materials")
            with open(mtl_path, 'w') as f:
                for col, name in materials.items():
                    f.write(f"newmtl {name}\n")
                    f.write(f"Kd {col[0]} {col[1]} {col[2]}\n")
                    f.write("d 1.0\n")
                    f.write("Ka 0.0 0.0 0.0\n")
                    f.write("Ks 0.0 0.0 0.0\n")
                    f.write("Ns 0.0\n")
                    f.write("illum 1\n")
            
            if os.path.exists(mtl_path):
                 log_debug(f"MTL file created: {mtl_path} ({os.path.getsize(mtl_path)} bytes)")
            else:
                 log_debug(f"ERROR: MTL file NOT created: {mtl_path}")
            
            t3 = time.time()
            log_debug(f"Exporting took {t3-t2:.2f}s")
            log_debug("Conversion successful.")
            return True
        except Exception as e:
            log_debug(f"Export error: {e}")
            traceback.print_exc()
            return False
            
    finally:
        # Cleanup temp file 
        if is_temp_copy and os.path.exists(safe_input_path):
            try:
                os.remove(safe_input_path)
                log_debug("Temporary copy removed.")
            except:
                pass

if __name__ == "__main__":
    in_p = os.environ.get("STEP2OBJ_INPUT") or (sys.argv[1] if len(sys.argv) > 1 else None)
    out_p = os.environ.get("STEP2OBJ_OUTPUT") or (sys.argv[2] if len(sys.argv) > 2 else None)
    if in_p and out_p:
        if convert_step_to_obj(in_p, out_p):
            sys.exit(0)
    sys.exit(1)
