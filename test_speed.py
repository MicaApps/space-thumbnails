# -*- coding: utf-8 -*-
import os
import shutil
import subprocess
import time
import re

# Source path with Chinese characters
source_path = r"assets\LGQGJ-00-00 杞瘋鎶涘厜鏈烘€昏.STEP"
temp_input = os.path.join(os.environ["TEMP"], "speed_test_input.step")
temp_output = os.path.join(os.environ["TEMP"], "test_speed.obj")
bat_file = os.path.abspath(r"tools\step2obj.bat")

def run_test(deflection, ang_deflection, relative, format_val, label):
    print(f"\nRunning Test: {label}")
    print(f"Params: Deflection={deflection}, Ang={ang_deflection}, Relative={relative}, Format={format_val}")
    
    env = os.environ.copy()
    env["STEP2OBJ_INPUT"] = os.path.abspath(temp_input)
    env["STEP2OBJ_OUTPUT"] = os.path.abspath(temp_output)
    env["STEP2OBJ_DEFLECTION"] = str(deflection)
    env["STEP2OBJ_ANG_DEFLECTION"] = str(ang_deflection)
    env["STEP2OBJ_RELATIVE"] = str(relative)
    env["STEP2OBJ_FORMAT"] = format_val
    
    start_time = time.time()
    # Use shell=True to run the bat file
    result = subprocess.run([bat_file], env=env, shell=True, capture_output=True)
    end_time = time.time()
    
    print(f"Total Time: {end_time - start_time:.2f}s")
    
    if result.returncode != 0:
        print(f"Error: Exit code {result.returncode}")
        print("STDERR:", result.stderr.decode('mbcs', errors='replace'))
    
    log_file = temp_output + ".log"
    if os.path.exists(log_file):
        with open(log_file, "r", encoding="utf-8", errors="replace") as f:
            log_content = f.read()
            
        meshing_time = re.search(r"Meshing took ([\d\.]+)s", log_content)
        if meshing_time:
            print(f"Internal Meshing Time: {meshing_time.group(1)}s")
            
        read_time = re.search(r"ReadFile took ([\d\.]+)s", log_content)
        if read_time:
            print(f"Internal ReadFile Time: {read_time.group(1)}s")

        tri_count = re.search(r"Total Triangles: (\d+)", log_content)
        if tri_count:
            print(f"Total Triangles: {tri_count.group(1)}")

        bbox_info = re.search(r"BBox: .* Diag: ([\d\.]+)", log_content)
        if bbox_info:
            print(f"BBox Diag: {bbox_info.group(1)}")

        exporting_time = re.search(r"Exporting took ([\d\.]+)s", log_content)
        if exporting_time:
            print(f"Internal Exporting Time: {exporting_time.group(1)}s")
            
        if "Error" in log_content:
            print("--- Log Errors ---")
            print(log_content)
            print("------------------")
        
        os.remove(log_file)
    else:
        print("Log file not found.")

    if os.path.exists(temp_output):
        os.remove(temp_output)
    mtl_file = temp_output.replace(".obj", ".mtl")
    if os.path.exists(mtl_file):
        os.remove(mtl_file)

def main():
    if not os.path.exists(source_path):
        print(f"Source file not found: {source_path}")
        return

    print(f"Copying {source_path} to {temp_input}...")
    shutil.copy2(source_path, temp_input)
    
    try:
        # Baseline (10.0, 0.5) OBJ
        run_test(10.0, 0.5, False, "OBJ", "Baseline OBJ (10.0, 0.5)")

        # Coarse (100.0, 0.8) OBJ
        run_test(100.0, 0.8, False, "OBJ", "Coarse OBJ (100.0, 0.8)")

        # Coarse (100.0, 0.8) STL
        run_test(100.0, 0.8, False, "STL", "Coarse STL (100.0, 0.8)")
        
    finally:
        if os.path.exists(temp_input):
            os.remove(temp_input)

if __name__ == "__main__":
    main()
