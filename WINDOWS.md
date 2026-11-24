# Windows x64 build

Open the **Windows app** workflow in GitHub Actions, choose **Run workflow**, then download **Tournament-Manager-Windows-x64** from the completed run. Extract and launch the `-setup.exe` installer. The installer installs WebView2 if needed. The bundled PDF helper includes its own .NET runtime.

The workflow compiles the Rust/Tauri application, tests the shared tournament/scoring/report code, builds the 32-bit controller helper and Windows PDF helper, then creates an NSIS installer. Installers are unsigned.

To build locally on Windows, install Rust stable (MSVC), Visual Studio C++ Build Tools including x86 tools, Node.js 22, and .NET SDK 8. Run `npm ci`, then `./tools/package-windows.ps1` in PowerShell. Output is in `dist/windows`.

## Supplied drivers

The bundled `v5field-windows.so` is a **32-bit Windows DLL** despite its extension. Its exports and the supplied Python wrapper match the existing V5 interface. A separate x86 helper loads it; the main Tauri application remains x64. The helper disables the field on input loss and has a two-second heartbeat watchdog. Physical controller behavior still needs Windows hardware testing.

The supplied CP210x USB driver and x86 VC runtime installers are preserved under `Drivers`, with their supplied license and release notes. Launch them from **Setup → Field hardware** only when needed. Windows asks for administrator permission. They are not installed automatically.

The CP210x release notes identify version **6.6 (2012)** and list XP through Windows 8. Compatibility with Windows 10/11 is unverified; do not replace a newer working system driver with this legacy package.

The Windows PDF exporter uses WebView2 printing with the same bundled font, purple accents, outlined tables, organization colors, and number-only elimination brackets. Sounds use Windows audio APIs.

For remote operation, start the event server on the host, then choose **Connect to a remote tournament** on the Windows welcome screen and enter the host address and tournament password. It connects to this remake's server, not the original TM protocol.
