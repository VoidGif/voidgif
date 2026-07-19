<#
  capture-cdp.ps1 — capture the two screenshots that need a runtime UI state:
    editor-light.png   (light theme forced on)
    export-dialog.png  (Export dialog opened)

  It launches ONE throwaway VoidGif instance with the WebView2 remote-debugging
  port enabled, then drives the page over the Chrome DevTools Protocol
  (via cdp-eval.mjs) to toggle the theme / open the dialog. This is renderer
  scripting through the debug protocol — NO OS mouse/keyboard input is injected —
  and each window is grabbed with PrintWindow (app window only, never the
  desktop). The instance is killed when done.

  NOTE: theme can't be pre-seeded via settings because Tauri resolves the config
  dir through the Windows Known-Folder API (ignores the APPDATA env var), so we
  flip it live over CDP instead.

  USAGE
    .\capture-cdp.ps1 -DemoProject "C:\path\to\demo.voidgif"
#>
[CmdletBinding()]
param(
    [string]$ExePath = "",
    [Parameter(Mandatory = $true)][string]$DemoProject,
    [string]$OutDir = "",
    [int]$Port = 9223,
    [int]$Width = 1600,
    [int]$Height = 900
)

$ErrorActionPreference = "Stop"
if ([string]::IsNullOrWhiteSpace($ExePath)) { $ExePath = (Resolve-Path (Join-Path $PSScriptRoot "..\..\src-tauri\target\release\voidgif.exe")).Path }
if ([string]::IsNullOrWhiteSpace($OutDir))  { $OutDir = $PSScriptRoot }
if (-not (Test-Path $ExePath))     { throw "voidgif.exe not found at $ExePath" }
if (-not (Test-Path $DemoProject)) { throw "demo project not found at $DemoProject" }
$cdp = Join-Path $PSScriptRoot "cdp-eval.mjs"

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win2 {
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int t, bool repaint);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

function Invoke-Cdp([string]$js) {
    $b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($js))
    $out = & node $cdp $Port $b64
    Write-Host "    cdp: $out" -ForegroundColor DarkGray
}

function Save-Window([IntPtr]$hwnd, [string]$outFile) {
    $r = New-Object Win2+RECT
    [Win2]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    [Win2]::PrintWindow($hwnd, $hdc, 2) | Out-Null   # PW_RENDERFULLCONTENT
    $g.ReleaseHdc($hdc); $g.Dispose()
    $dst = Join-Path $OutDir $outFile
    $bmp.Save($dst, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "    saved $dst (${w}x${h})" -ForegroundColor Green
}

# Isolate the WebView2 user-data folder: sharing the owner's already-running
# instance's folder while adding --remote-debugging-port makes WebView2 fail
# with 0x8007139F (conflicting browser args for one user-data dir).
$wv2 = Join-Path $env:TEMP ("vg-cdp-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $wv2 | Out-Null

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $ExePath
$psi.Arguments = '"' + $DemoProject + '"'
$psi.UseShellExecute = $false
$psi.EnvironmentVariables["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = "--remote-debugging-port=$Port"
$psi.EnvironmentVariables["WEBVIEW2_USER_DATA_FOLDER"] = $wv2

Write-Host "==> launching (CDP port $Port)" -ForegroundColor Cyan
$proc = [System.Diagnostics.Process]::Start($psi)
try {
    $hwnd = [IntPtr]::Zero
    for ($i = 0; $i -lt 60; $i++) {
        Start-Sleep -Milliseconds 300
        $proc.Refresh()
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { $hwnd = $proc.MainWindowHandle; break }
        if ($proc.HasExited) { throw "process exited early ($($proc.ExitCode))" }
    }
    if ($hwnd -eq [IntPtr]::Zero) { throw "no main window appeared" }

    Start-Sleep -Seconds 3                                   # webview attach + session load
    [Win2]::MoveWindow($hwnd, 0, 0, $Width, $Height, $true) | Out-Null
    Start-Sleep -Seconds 3                                   # reflow at final size

    # Wait until the CDP endpoint is serving a page target.
    for ($i = 0; $i -lt 20; $i++) {
        try { $j = Invoke-RestMethod "http://127.0.0.1:$Port/json" -TimeoutSec 2; if ($j) { break } } catch {}
        Start-Sleep -Milliseconds 500
    }

    Write-Host "==> editor-light.png" -ForegroundColor Cyan
    Invoke-Cdp "document.documentElement.classList.add('light'); localStorage.setItem('vg-theme','light'); 'light-on'"
    Start-Sleep -Milliseconds 1500
    Save-Window $hwnd "editor-light.png"

    Write-Host "==> export-dialog.png" -ForegroundColor Cyan
    # Language-agnostic: the Export button is the right-most button in the top
    # toolbar (matching by text fails when the UI is not English).
    Invoke-Cdp "document.documentElement.classList.remove('light'); (function(){var bs=Array.prototype.slice.call(document.querySelectorAll('button')).filter(function(b){var r=b.getBoundingClientRect(); return r.width>0 && r.top>=0 && r.top<90;}); bs.sort(function(a,b){return a.getBoundingClientRect().right-b.getBoundingClientRect().right;}); var b=bs[bs.length-1]; if(b){b.click(); return 'clicked';} return 'notfound';})()"
    Start-Sleep -Milliseconds 1800
    Save-Window $hwnd "export-dialog.png"
}
finally {
    & taskkill /PID $proc.Id /T /F *> $null
    Start-Sleep -Milliseconds 500
    Remove-Item $wv2 -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host "Done." -ForegroundColor Cyan
