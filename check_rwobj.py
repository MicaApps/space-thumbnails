try:
    from OCP.RWObj import RWObj_CafWriter
    print("RWObj_CafWriter is available")
except ImportError:
    print("RWObj_CafWriter is NOT available")
except Exception as e:
    print(f"Error: {e}")
