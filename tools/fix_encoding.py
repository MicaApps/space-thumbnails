import codecs

with open('debug_run.ps1', 'r', encoding='utf-8') as f:
    content = f.read()

with open('debug_run_fixed.ps1', 'w', encoding='utf-8-sig') as f:
    f.write(content)

print("Created debug_run_fixed.ps1 with BOM")
