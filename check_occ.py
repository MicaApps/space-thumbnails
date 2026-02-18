from OCP.StlAPI import StlAPI_Writer
from OCP.TopoDS import TopoDS_Shape
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox

print("StlAPI_Writer is available.")

box = BRepPrimAPI_MakeBox(10.0, 10.0, 10.0).Shape()
writer = StlAPI_Writer()
writer.Write(box, "test.stl")
print("STL write test passed.")
