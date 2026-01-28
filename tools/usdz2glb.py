import sys
import os
import json
import struct
import math
import zipfile
from pxr import Usd, UsdGeom, UsdShade, Sdf, Gf, Vt

# GLB Constants
GLTF_VERSION = 2
GLTF_MAGIC = 0x46546C67
JSON_CHUNK_TYPE = 0x4E4F534A
BIN_CHUNK_TYPE = 0x004E4942

# Component Types
BYTE = 5120
UNSIGNED_BYTE = 5121
SHORT = 5122
UNSIGNED_SHORT = 5123
UNSIGNED_INT = 5125
FLOAT = 5126

# Buffer Target
ARRAY_BUFFER = 34962
ELEMENT_ARRAY_BUFFER = 34963

class GlbWriter:
    def __init__(self):
        self.buffer = bytearray()
        self.json = {
            "asset": {"version": "2.0", "generator": "space-thumbnails-usdz2glb"},
            "scenes": [{"nodes": []}],
            "scene": 0,
            "nodes": [],
            "meshes": [],
            "buffers": [{"byteLength": 0}],
            "bufferViews": [],
            "accessors": [],
            "materials": [],
            "textures": [],
            "images": [],
            "samplers": [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}] # Default sampler
        }
        
    def align_buffer(self, alignment=4):
        padding = (alignment - (len(self.buffer) % alignment)) % alignment
        self.buffer.extend(b'\x00' * padding)
        
    def add_buffer_view(self, data, target=None):
        self.align_buffer(4)
        byte_offset = len(self.buffer)
        byte_length = len(data)
        self.buffer.extend(data)
        
        view = {
            "buffer": 0,
            "byteOffset": byte_offset,
            "byteLength": byte_length
        }
        if target:
            view["target"] = target
            
        self.json["bufferViews"].append(view)
        return len(self.json["bufferViews"]) - 1
        
    def add_accessor(self, buffer_view_idx, component_type, count, type_str, min_val=None, max_val=None):
        accessor = {
            "bufferView": buffer_view_idx,
            "componentType": component_type,
            "count": count,
            "type": type_str,
            "byteOffset": 0
        }
        if min_val is not None:
            accessor["min"] = min_val
        if max_val is not None:
            accessor["max"] = max_val
            
        self.json["accessors"].append(accessor)
        return len(self.json["accessors"]) - 1

    def add_image(self, data, mime_type):
        view_idx = self.add_buffer_view(data)
        image = {
            "bufferView": view_idx,
            "mimeType": mime_type
        }
        self.json["images"].append(image)
        return len(self.json["images"]) - 1

    def add_texture(self, image_idx):
        texture = {
            "sampler": 0,
            "source": image_idx
        }
        self.json["textures"].append(texture)
        return len(self.json["textures"]) - 1

    def add_material(self, name, pbr_data):
        # pbr_data: {baseColorFactor, baseColorTexture, metallicFactor, roughnessFactor, metallicRoughnessTexture, normalTexture, occlusionTexture, emissiveTexture, emissiveFactor}
        mat = {
            "name": name,
            "pbrMetallicRoughness": {},
            "doubleSided": True
        }
        
        pbr = mat["pbrMetallicRoughness"]
        
        if "baseColorFactor" in pbr_data:
            pbr["baseColorFactor"] = pbr_data["baseColorFactor"]
        else:
            pbr["baseColorFactor"] = [1.0, 1.0, 1.0, 1.0]
            
        if "baseColorTexture" in pbr_data:
            pbr["baseColorTexture"] = {"index": pbr_data["baseColorTexture"]}
            
        if "metallicFactor" in pbr_data:
            pbr["metallicFactor"] = pbr_data["metallicFactor"]
        else:
            pbr["metallicFactor"] = 0.0 # Default non-metal
            
        if "roughnessFactor" in pbr_data:
            pbr["roughnessFactor"] = pbr_data["roughnessFactor"]
        else:
            pbr["roughnessFactor"] = 0.5

        if "metallicRoughnessTexture" in pbr_data:
            pbr["metallicRoughnessTexture"] = {"index": pbr_data["metallicRoughnessTexture"]}

        if "normalTexture" in pbr_data:
            mat["normalTexture"] = {"index": pbr_data["normalTexture"]}
            
        if "occlusionTexture" in pbr_data:
            mat["occlusionTexture"] = {"index": pbr_data["occlusionTexture"]}
            
        if "emissiveTexture" in pbr_data:
            mat["emissiveTexture"] = {"index": pbr_data["emissiveTexture"]}
            
        if "emissiveFactor" in pbr_data:
            mat["emissiveFactor"] = pbr_data["emissiveFactor"]

        self.json["materials"].append(mat)
        return len(self.json["materials"]) - 1

    def add_mesh(self, name, primitives):
        mesh = {
            "name": name,
            "primitives": primitives
        }
        self.json["meshes"].append(mesh)
        return len(self.json["meshes"]) - 1

    def add_node(self, name, mesh_idx, matrix=None):
        node = {
            "name": name,
            "mesh": mesh_idx
        }
        if matrix:
            node["matrix"] = matrix
            
        self.json["nodes"].append(node)
        idx = len(self.json["nodes"]) - 1
        self.json["scenes"][0]["nodes"].append(idx)
        return idx

    def write(self, path):
        self.json["buffers"][0]["byteLength"] = len(self.buffer)
        json_str = json.dumps(self.json, separators=(',', ':'))
        json_bytes = json_str.encode('utf-8')
        
        padding = (4 - (len(json_bytes) % 4)) % 4
        json_bytes += b' ' * padding
        
        bin_padding = (4 - (len(self.buffer) % 4)) % 4
        self.buffer.extend(b'\x00' * bin_padding)
        
        total_length = 12 + 8 + len(json_bytes) + 8 + len(self.buffer)
        
        with open(path, 'wb') as f:
            f.write(struct.pack('<III', GLTF_MAGIC, GLTF_VERSION, total_length))
            f.write(struct.pack('<II', len(json_bytes), JSON_CHUNK_TYPE))
            f.write(json_bytes)
            f.write(struct.pack('<II', len(self.buffer), BIN_CHUNK_TYPE))
            f.write(self.buffer)
        print(f"Exported GLB to {path} ({total_length} bytes)")

def resolve_texture(stage, input_attr, usdz_zip, writer, texture_cache):
    if not input_attr:
        return None

    # Check for connection
    if input_attr.GetAttr().HasAuthoredConnections():
        sources, _ = input_attr.GetConnectedSources()
        for src in sources:
            # Check if source is a UsdUVTexture
            # src is UsdShade.ConnectionSourceInfo
            if not hasattr(src, 'source'):
                continue
            shader_prim = src.source.GetPrim()
            shader = UsdShade.Shader(shader_prim)
            if not shader: continue
            
            id_attr = shader.GetIdAttr()
            if id_attr and id_attr.Get() == "UsdUVTexture":
                file_input = shader.GetInput("file")
                if file_input:
                    asset_path = file_input.Get()
                    if asset_path:
                        path_str = str(asset_path.path)
                        # Remove leading ./ if present
                        if path_str.startswith("./"):
                            path_str = path_str[2:]
                        
                        # Cache check
                        if path_str in texture_cache:
                            return texture_cache[path_str]

                        # Read from zip
                        try:
                            # Try exact match first
                            data = None
                            try:
                                data = usdz_zip.read(path_str)
                            except KeyError:
                                # Try to find case-insensitive match or path fix
                                # List all files
                                all_files = usdz_zip.namelist()
                                for f in all_files:
                                    if f.endswith(path_str) or f.lower() == path_str.lower():
                                        data = usdz_zip.read(f)
                                        break
                            
                            if data:
                                mime = "image/jpeg"
                                if path_str.lower().endswith(".png"):
                                    mime = "image/png"
                                
                                img_idx = writer.add_image(data, mime)
                                tex_idx = writer.add_texture(img_idx)
                                texture_cache[path_str] = tex_idx
                                return tex_idx
                        except Exception as e:
                            print(f"Failed to read texture {path_str}: {e}")
    return None

def convert_usdz_to_glb(usdz_path, glb_path):
    print(f"Opening stage: {usdz_path}")
    stage = Usd.Stage.Open(usdz_path)
    if not stage:
        print(f"Error: Could not open {usdz_path}")
        sys.exit(1)

    usdz_zip = zipfile.ZipFile(usdz_path, 'r')
    writer = GlbWriter()
    
    # Caches
    texture_cache = {} # path -> texture_index
    material_map = {} # path -> material_index
    
    time = Usd.TimeCode.Default()
    xform_cache = UsdGeom.XformCache(time)

    for prim in stage.Traverse():
        if prim.IsA(UsdGeom.Mesh):
            mesh = UsdGeom.Mesh(prim)
            
            # --- Geometry ---
            points = mesh.GetPointsAttr().Get(time)
            if not points: continue
            
            # Transform to world
            transform = xform_cache.GetLocalToWorldTransform(prim)
            
            vertices = []
            min_pos = [float('inf')] * 3
            max_pos = [float('-inf')] * 3
            
            for p in points:
                tp = transform.Transform(p)
                v = [tp[0], tp[1], tp[2]]
                vertices.append(v)
                for i in range(3):
                    min_pos[i] = min(min_pos[i], v[i])
                    max_pos[i] = max(max_pos[i], v[i])

            # Normals
            final_normals = None
            normals_attr = mesh.GetNormalsAttr().Get(time)
            if normals_attr and len(normals_attr) == len(points):
                final_normals = []
                for n in normals_attr:
                    tn = transform.TransformDir(n)
                    length = math.sqrt(tn[0]*tn[0] + tn[1]*tn[1] + tn[2]*tn[2])
                    if length > 0:
                        final_normals.append([tn[0]/length, tn[1]/length, tn[2]/length])
                    else:
                        final_normals.append([0, 1, 0])

            # UVs
            final_uvs = None
            primvars = UsdGeom.PrimvarsAPI(mesh)
            pv_st = primvars.GetPrimvar("st")
            if not pv_st: pv_st = primvars.GetPrimvar("uv")
            
            if pv_st:
                # If indexed, we should flatten. ComputeFlattened handles this.
                # However, if topology is face-varying (different UVs for same vertex on different faces),
                # we need to split vertices. 
                # For simplicity in this script, we only handle vertex-interpolation or same-count UVs.
                # If UV count != Vertex count, this simple script might produce artifacts or crash.
                # A robust solution requires re-indexing.
                
                # Check interpolation
                interp = pv_st.GetInterpolation()
                uv_data = pv_st.ComputeFlattened(time)
                
                if uv_data and len(uv_data) == len(points):
                    final_uvs = []
                    for uv in uv_data:
                        final_uvs.append([uv[0], 1.0 - uv[1]]) # Flip V for GLTF
                elif uv_data and len(uv_data) > len(points):
                     # Likely face-varying.
                     # We are not handling splitting vertices here to keep it simple.
                     # We'll just take the first N or skip.
                     # Ideally we should skip to avoid garbage.
                     print(f"Warning: UV count {len(uv_data)} != Point count {len(points)}. Skipping UVs.")
                     pass

            # Indices
            indices = []
            face_counts = mesh.GetFaceVertexCountsAttr().Get(time)
            face_indices = mesh.GetFaceVertexIndicesAttr().Get(time)
            idx_ptr = 0
            for count in face_counts:
                base = face_indices[idx_ptr]
                for i in range(1, count - 1):
                    indices.append(base)
                    indices.append(face_indices[idx_ptr + i])
                    indices.append(face_indices[idx_ptr + i + 1])
                idx_ptr += count

            # --- Buffers ---
            pos_bytes = bytearray()
            for v in vertices:
                pos_bytes.extend(struct.pack('<fff', v[0], v[1], v[2]))
            pos_view = writer.add_buffer_view(pos_bytes, ARRAY_BUFFER)
            pos_acc = writer.add_accessor(pos_view, FLOAT, len(vertices), "VEC3", min_pos, max_pos)
            
            attributes = {"POSITION": pos_acc}
            
            if final_normals:
                norm_bytes = bytearray()
                for n in final_normals:
                    norm_bytes.extend(struct.pack('<fff', n[0], n[1], n[2]))
                norm_view = writer.add_buffer_view(norm_bytes, ARRAY_BUFFER)
                norm_acc = writer.add_accessor(norm_view, FLOAT, len(final_normals), "VEC3")
                attributes["NORMAL"] = norm_acc

            if final_uvs:
                uv_bytes = bytearray()
                for uv in final_uvs:
                    uv_bytes.extend(struct.pack('<ff', uv[0], uv[1]))
                uv_view = writer.add_buffer_view(uv_bytes, ARRAY_BUFFER)
                uv_acc = writer.add_accessor(uv_view, FLOAT, len(final_uvs), "VEC2")
                attributes["TEXCOORD_0"] = uv_acc

            idx_bytes = bytearray()
            if len(vertices) < 65536:
                for idx in indices:
                    idx_bytes.extend(struct.pack('<H', idx))
                idx_comp_type = UNSIGNED_SHORT
            else:
                for idx in indices:
                    idx_bytes.extend(struct.pack('<I', idx))
                idx_comp_type = UNSIGNED_INT
            idx_view = writer.add_buffer_view(idx_bytes, ELEMENT_ARRAY_BUFFER)
            idx_acc = writer.add_accessor(idx_view, idx_comp_type, len(indices), "SCALAR")

            # --- Material ---
            mat_idx = None
            
            # 1. Try UsdShade binding
            binding_api = UsdShade.MaterialBindingAPI(prim)
            bound_material = binding_api.ComputeBoundMaterial()[0]
            
            if bound_material:
                mat_path = bound_material.GetPath()
                if mat_path in material_map:
                    mat_idx = material_map[mat_path]
                else:
                    # Parse Material
                    pbr_data = {}
                    
                    # Find Surface Shader
                    shader = bound_material.ComputeSurfaceSource()[0]
                    if shader:
                        id_attr = shader.GetIdAttr()
                        # Support UsdPreviewSurface
                        if id_attr.Get() == "UsdPreviewSurface":
                            # Base Color
                            base_color = shader.GetInput("diffuseColor")
                            if base_color:
                                # Check value (Get() returns None if no value)
                                val = base_color.Get()
                                if val is not None:
                                    pbr_data["baseColorFactor"] = [val[0], val[1], val[2], 1.0]
                                tex = resolve_texture(stage, base_color, usdz_zip, writer, texture_cache)
                                if tex is not None:
                                    pbr_data["baseColorTexture"] = tex

                            # Metallic
                            metallic = shader.GetInput("metallic")
                            if metallic:
                                val = metallic.Get()
                                if val is not None:
                                    pbr_data["metallicFactor"] = val
                                tex = resolve_texture(stage, metallic, usdz_zip, writer, texture_cache)
                                if tex is not None:
                                    pbr_data["metallicRoughnessTexture"] = tex 

                            # Roughness
                            roughness = shader.GetInput("roughness")
                            if roughness:
                                val = roughness.Get()
                                if val is not None:
                                    pbr_data["roughnessFactor"] = val

                            # Normal
                            normal = shader.GetInput("normal")
                            if normal:
                                tex = resolve_texture(stage, normal, usdz_zip, writer, texture_cache)
                                if tex is not None:
                                    pbr_data["normalTexture"] = tex

                            # Occlusion
                            occlusion = shader.GetInput("occlusion")
                            if occlusion:
                                tex = resolve_texture(stage, occlusion, usdz_zip, writer, texture_cache)
                                if tex is not None:
                                    pbr_data["occlusionTexture"] = tex
                                    
                            # Emissive
                            emissive = shader.GetInput("emissiveColor")
                            if emissive:
                                val = emissive.Get()
                                if val is not None:
                                    pbr_data["emissiveFactor"] = [val[0], val[1], val[2]]
                                tex = resolve_texture(stage, emissive, usdz_zip, writer, texture_cache)
                                if tex is not None:
                                    pbr_data["emissiveTexture"] = tex

                    mat_idx = writer.add_material(bound_material.GetPrim().GetName(), pbr_data)
                    material_map[mat_path] = mat_idx

            # 2. Fallback to DisplayColor if no material
            if mat_idx is None:
                display_color = mesh.GetDisplayColorAttr().Get(time)
                if display_color:
                    c = display_color[0]
                    pbr_data = {"baseColorFactor": [c[0], c[1], c[2], 1.0]}
                    mat_idx = writer.add_material("displayColor", pbr_data)

            primitive = {
                "attributes": attributes,
                "indices": idx_acc,
                "mode": 4
            }
            if mat_idx is not None:
                primitive["material"] = mat_idx

            mesh_idx = writer.add_mesh(prim.GetName(), [primitive])
            writer.add_node(prim.GetName(), mesh_idx)

    writer.write(glb_path)
    usdz_zip.close()

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python usdz2glb.py input.usdz output.glb")
        sys.exit(1)
    
    convert_usdz_to_glb(sys.argv[1], sys.argv[2])
