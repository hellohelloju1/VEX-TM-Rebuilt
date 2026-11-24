$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
function Check-Native { if ($LASTEXITCODE -ne 0) { throw "Build command failed with exit code $LASTEXITCODE" } }
# The vendor field-control DLL is 32-bit; isolate it in a 32-bit helper.
$vswhere = "${env:ProgramFiles(x86)}/Microsoft Visual Studio/Installer/vswhere.exe"
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (!$vs) { throw 'Install Visual Studio C++ Build Tools with the x86 toolchain' }
$vcvars = Join-Path $vs 'VC/Auxiliary/Build/vcvars32.bat'
$compile = '"{0}" && cl /nologo /std:c++17 /EHsc /MT /O2 native\v5_bridge_windows.cpp /Fe:native\v5-bridge.exe /Fo:native\v5-bridge.obj' -f $vcvars
& cmd.exe /d /s /c $compile
Check-Native
# Build the PDF helper with its .NET runtime included; no .NET install required by users.
dotnet publish native/windows-pdf/ReportPdf.csproj -c Release -r win-x64 --self-contained true -o native/report-pdf-win
Check-Native
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Force icons | Out-Null
$image = [System.Drawing.Image]::FromFile((Join-Path $PWD 'assets/v5.png'))
$canvas = [System.Drawing.Bitmap]::new(256,256)
$graphics = [System.Drawing.Graphics]::FromImage($canvas)
$graphics.Clear([System.Drawing.Color]::White)
$graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$scale = [Math]::Min(224.0 / $image.Width, 224.0 / $image.Height)
$width = [int]($image.Width * $scale); $height = [int]($image.Height * $scale)
$graphics.DrawImage($image,[int]((256-$width)/2),[int]((256-$height)/2),$width,$height)
$canvas.Save((Join-Path $PWD 'icons/source.png'),[System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose(); $canvas.Dispose(); $image.Dispose()
npx --yes --package @tauri-apps/cli@2 tauri icon icons/source.png --output icons
Check-Native
npx --yes --package @tauri-apps/cli@2 tauri build --target x86_64-pc-windows-msvc --bundles nsis
Check-Native
$destination = Join-Path $PWD 'dist/windows'
New-Item -ItemType Directory -Force $destination | Out-Null
$installers = @(Get-ChildItem target/x86_64-pc-windows-msvc/release/bundle/nsis/*-setup.exe)
if (!$installers.Count) { throw 'Windows installer was not produced' }
$installers | Copy-Item -Destination $destination
Get-ChildItem $destination | Get-FileHash -Algorithm SHA256 | Format-Table
