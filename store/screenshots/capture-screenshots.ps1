<#
  capture-screenshots.ps1 — produce Store-ready screenshots of VoidGif.

  HOW IT STAYS OUT OF YOUR WAY
    * Each shot launches a SEPARATE, throwaway VoidGif instance with an isolated
      APPDATA/TEMP so your real settings and the running app are untouched.
    * The window is captured with PrintWindow(PW_RENDERFULLCONTENT) — only the
      app window is grabbed, never the desktop — and can be captured while moved
      off to the side. No mouse/keyboard input is injected.
    * Every instance it starts, it kills (process tree) before moving on.

  OUTPUT (1600x900 PNG, exceeds the 1366x768 Store minimum):
    home-dark.png, editor-dark.png, editor-light.png
    (export-dialog.png is added by capture-export.ps1, which needs CDP.)

  USAGE
    .\capture-screenshots.ps1 -DemoProject "C:\path\to\demo.voidgif"
#>
[CmdletBinding()]
param(
    [string]$ExePath = "",
    [string]$DemoProject = "",
    [string]$OutDir = "",
    [int]$Width = 1600,
    [int]$Height = 900,
    # By default the window is captured on-screen at the top-left corner (this is
    # what makes WebView2 reflow + composite correctly). -OffScreen hides it at
    # the cost of a possible mis-laid-out capture on some GPUs.
    [switch]$OffScreen
)

$ErrorActionPreference = "Stop"
if ([string]::IsNullOrWhiteSpace($ExePath))  { $ExePath = (Resolve-Path (Join-Path $PSScriptRoot "..\..\src-tauri\target\release\voidgif.exe")).Path }
if ([string]::IsNullOrWhiteSpace($OutDir))   { $OutDir = $PSScriptRoot }
if (-not (Test-Path $ExePath)) { throw "voidgif.exe not found at $ExePath — run cargo build --release first." }

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win {
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int t, bool repaint);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

function Capture-Shot {
    param(
        [string]$Theme,       # "dark" | "light"
        [string]$ArgvPath,    # "" for Home, else a .voidgif path
        [string]$OutFile
    )
    $iso = Join-Path $env:TEMP ("vg-shot-" + [guid]::NewGuid().ToString("N"))
    $cfgDir = Join-Path $iso "Roaming\com.voidgif.desktop"
    New-Item -ItemType Directory -Force -Path $cfgDir, (Join-Path $iso "Local"), (Join-Path $iso "Temp"), (Join-Path $iso "WV2") | Out-Null
    # Seed settings so the app skips onboarding and uses the theme/lang we want.
    $settings = '{"theme":"' + $Theme + '","language":"en","defaultFps":30,"defaultCursor":true}'
    Set-Content -Path (Join-Path $cfgDir "settings.json") -Value $settings -Encoding UTF8

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $ExePath
    if (-not [string]::IsNullOrWhiteSpace($ArgvPath)) { $psi.Arguments = '"' + $ArgvPath + '"' }
    $psi.UseShellExecute = $false
    $psi.EnvironmentVariables["APPDATA"]      = (Join-Path $iso "Roaming")
    $psi.EnvironmentVariables["LOCALAPPDATA"] = (Join-Path $iso "Local")
    $psi.EnvironmentVariables["TEMP"]         = (Join-Path $iso "Temp")
    $psi.EnvironmentVariables["TMP"]          = (Join-Path $iso "Temp")
    $psi.EnvironmentVariables["WEBVIEW2_USER_DATA_FOLDER"] = (Join-Path $iso "WV2")

    Write-Host "==> launching for $OutFile (theme=$Theme)" -ForegroundColor Cyan
    $proc = [System.Diagnostics.Process]::Start($psi)
    try {
        # Wait for the main window to exist.
        $hwnd = [IntPtr]::Zero
        for ($i = 0; $i -lt 60; $i++) {
            Start-Sleep -Milliseconds 300
            $proc.Refresh()
            if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { $hwnd = $proc.MainWindowHandle; break }
            if ($proc.HasExited) { throw "process exited early (code $($proc.ExitCode))" }
        }
        if ($hwnd -eq [IntPtr]::Zero) { throw "no main window appeared" }

        # Let the WebView2 child attach + first-paint at the initial size BEFORE
        # resizing, otherwise it won't reflow to the new client size.
        Start-Sleep -Seconds 3
        $x = if ($OffScreen) { -3200 } else { 0 }
        [Win]::MoveWindow($hwnd, $x, 0, $Width, $Height, $true) | Out-Null
        Start-Sleep -Seconds 4   # reflow + session load + repaint at final size

        $r = New-Object Win+RECT
        [Win]::GetWindowRect($hwnd, [ref]$r) | Out-Null
        $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
        if ($w -le 0 -or $h -le 0) { throw "bad window rect ${w}x${h}" }

        $bmp = New-Object System.Drawing.Bitmap $w, $h
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $hdc = $g.GetHdc()
        # PW_RENDERFULLCONTENT = 0x2 — required to capture WebView2 content.
        [Win]::PrintWindow($hwnd, $hdc, 2) | Out-Null
        $g.ReleaseHdc($hdc); $g.Dispose()
        $dst = Join-Path $OutDir $OutFile
        $bmp.Save($dst, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
        Write-Host "    saved $dst (${w}x${h})" -ForegroundColor Green
    }
    finally {
        # Kill the whole tree (host + msedgewebview2 children).
        & taskkill /PID $proc.Id /T /F *> $null
        Start-Sleep -Milliseconds 500
        Remove-Item $iso -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Capture-Shot -Theme "dark"  -ArgvPath ""           -OutFile "home-dark.png"
if (-not [string]::IsNullOrWhiteSpace($DemoProject) -and (Test-Path $DemoProject)) {
    Capture-Shot -Theme "dark"  -ArgvPath $DemoProject -OutFile "editor-dark.png"
    Capture-Shot -Theme "light" -ArgvPath $DemoProject -OutFile "editor-light.png"
} else {
    Write-Warning "No -DemoProject given; skipping editor shots. Generate one with:`n  cargo run --release --example make_test_project --manifest-path src-tauri\Cargo.toml -- demo.voidgif 36 480 300"
}
Write-Host "Done." -ForegroundColor Cyan
