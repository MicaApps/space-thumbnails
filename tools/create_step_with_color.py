
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox
from OCP.STEPCAFControl import STEPCAFControl_Writer
from OCP.TDocStd import TDocStd_Document
from OCP.XCAFDoc import XCAFDoc_DocumentTool, XCAFDoc_ColorTool, XCAFDoc_ColorSurf
from OCP.XCAFApp import XCAFApp_Application
from OCP.TCollection import TCollection_ExtendedString
from OCP.Quantity import Quantity_Color, Quantity_TOC_RGB
from OCP.STEPControl import STEPControl_AsIs

def create_colored_step(filename):
    # Create a document
    app = XCAFApp_Application.GetApplication_s()
    doc = TDocStd_Document(TCollection_ExtendedString("MDTV-XCAF"))
    app.NewDocument(TCollection_ExtendedString("MDTV-XCAF"), doc)
    
    # Get tools
    shape_tool = XCAFDoc_DocumentTool.ShapeTool_s(doc.Main())
    color_tool = XCAFDoc_DocumentTool.ColorTool_s(doc.Main())
    
    # Create a box
    box = BRepPrimAPI_MakeBox(10, 10, 10).Shape()
    
    # Add shape to document
    label = shape_tool.AddShape(box)
    
    # Create a Green color
    green = Quantity_Color(0.0, 1.0, 0.0, Quantity_TOC_RGB)
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopAbs import TopAbs_FACE
    
    # Assign color to the shape (Surface color)
    color_tool.SetColor(label, green, XCAFDoc_ColorSurf)
    
    # Assign color to faces
    # exp = TopExp_Explorer(box, TopAbs_FACE)
    # while exp.More():
    #     face = exp.Current()
    #     label_face = shape_tool.AddSubShape(label, face)
    #     color_tool.SetColor(label_face, red, XCAFDoc_ColorSurf)
        
    #     # Verify
    #     col_verify = Quantity_Color()
    #     shape_face = shape_tool.GetShape_s(label_face)
    #     if color_tool.GetColor(shape_face, XCAFDoc_ColorSurf, col_verify):
    #         print(f"Verified color on face: {col_verify.Red()}")
    #     else:
    #         print("Verification failed!")
            
    #     exp.Next()
    
    # Write to STEP
    writer = STEPCAFControl_Writer()
    writer.Transfer(doc, STEPControl_AsIs)
    
    status = writer.Write(filename)
    if status == 1: # IFSelect_RetDone
        print(f"Successfully wrote {filename}")
    else:
        print(f"Failed to write {filename} (status: {status})")

if __name__ == "__main__":
    create_colored_step("assets/generated_green.stp")
