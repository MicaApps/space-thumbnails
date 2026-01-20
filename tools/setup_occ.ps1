$ScriptDir = $PSScriptRoot
$PythonDir = Join-Path $ScriptDir "python"
$PythonZip = Join-Path $ScriptDir "python-embed.zip"
$GetPip = Join-Path $ScriptDir "get-pip.py"

# 1. Clean old
if (Test-Path $PythonDir) { Remove-Item -Recurse -Force $PythonDir }
New-Item -ItemType Directory -Path $PythonDir | Out-Null

# 2. Download Python Embed (3.10.11)
$Url = "https://www.python.org/ftp/python/3.10.11/python-3.10.11-embed-amd64.zip"
Write-Host "Downloading Python Embed from $Url..."
# Use TLS 1.2
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Invoke-WebRequest -Uri $Url -OutFile $PythonZip

# 3. Unzip
Write-Host "Unzipping..."
Expand-Archive -Path $PythonZip -DestinationPath $PythonDir -Force
Remove-Item $PythonZip

# 4. Enable site-packages (import for pip)
# Edit python310._pth to uncomment "import site"
$PthFile = Join-Path $PythonDir "python310._pth"
$Content = Get-Content $PthFile
$Content = $Content -replace "#import site", "import site"
Set-Content $PthFile $Content

# 5. Install pip
Write-Host "Downloading get-pip.py..."
Invoke-WebRequest -Uri "https://bootstrap.pypa.io/get-pip.py" -OutFile $GetPip
Write-Host "Installing pip..."
& "$PythonDir\python.exe" $GetPip

# 6. Install OCP and Numpy
Write-Host "Installing cadquery-ocp and numpy..."
& "$PythonDir\python.exe" -m pip install cadquery-ocp numpy

# Clean up get-pip
if (Test-Path $GetPip) { Remove-Item $GetPip }

Write-Host "Done! Python environment is ready at $PythonDir"
