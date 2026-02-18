try:
    from OCP.RWGltf import RWGltf_CafWriter
    print("RWGltf_CafWriter: Found")
except ImportError:
    print("RWGltf_CafWriter: Not Found")

try:
    from OCP.RWObj import RWObj_CafWriter
    print("RWObj_CafWriter: Found")
except ImportError:
    print("RWObj_CafWriter: Not Found")
