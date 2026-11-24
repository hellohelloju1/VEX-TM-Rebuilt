$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
function Check-Native { if ($LASTEXITCODE -ne 0) { throw "Helper compilation failed: $LASTEXITCODE" } }
# The vendor field-control DLL is 32-bit; isolate it in a 32-bit helper.
$vswhere = "${env:ProgramFiles(x86)}/Microsoft Visual Studio/Installer/vswhere.exe"
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (!$vs) { throw 'Install Visual Studio C++ Build Tools with the x86 toolchain' }
$vcvars = Join-Path $vs 'VC/Auxiliary/Build/vcvars32.bat'
$compile = '"{0}" && cl /nologo /std:c++17 /EHsc /MT /O2 native\v5_bridge_windows.cpp /Fe:native\v5-bridge.exe /Fo:native\v5-bridge.obj' -f $vcvars
& cmd.exe /d /s /c $compile
Check-Native
