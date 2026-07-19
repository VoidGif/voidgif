<#
  build-msix.ps1 — package the VoidGif release .exe into an MSIX for the
  Microsoft Store. Windows PowerShell 5.1 safe.

  WHAT IT DOES
    1. Verifies the release build exists (target\release\voidgif.exe).
    2. Fills the three Partner Center identity values into a COPY of
       AppxManifest.xml (the template file is never modified).
    3. Generates every required tile / logo PNG from src-tauri\icons\icon.png
       (512x512 master) with high-quality scaling.
    4. Builds resources.pri (makepri) and the .msix (makeappx) from the
       Windows 10 SDK you already have installed.

  SIGNING — READ THIS
    Do NOT sign the .msix you upload to the Store. Microsoft re-signs every
    MSIX package with a Microsoft-trusted certificate during certification, so
    a signature you add would just be replaced. Upload the UNSIGNED .msix that
    this script produces.
    The optional -SelfSignForLocalTest switch signs a throwaway copy ONLY so
    you can sideload-install and eyeball the app on THIS machine. That signed
    copy is for local testing — never upload it.

  TYPICAL USAGE (after `cargo build --release` in src-tauri)
    # values copied from Partner Center ▸ Product identity:
    .\build-msix.ps1 `
        -IdentityName        "1234ABCD.VoidGif" `
        -Publisher           "CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX" `
        -PublisherDisplayName "Your Seller Name"

  Result:  store\msix\VoidGif_0.1.0.0_x64.msix   ← upload THIS to Partner Center.
#>

[CmdletBinding()]
param(
    # The three values from Partner Center ▸ your app ▸ Product management ▸
    # Product identity. If you leave these empty you must instead edit the
    # placeholders in AppxManifest.xml by hand before running.
    [string]$IdentityName = "",
    [string]$Publisher = "",
    [string]$PublisherDisplayName = "",

    # Package version — keep the 4th field 0; bump for each new submission.
    [string]$Version = "0.1.0.0",

    # Override the source .exe if it is not at the default release path.
    [string]$ExePath = "",

    # Override the output .msix path.
    [string]$Output = "",

    # Skip resources.pri generation (package still installs; not recommended).
    [switch]$SkipPri,

    # Sign a *_signed.msix copy with a throwaway self-signed cert for LOCAL
    # sideload testing only. Never upload the signed copy.
    [switch]$SelfSignForLocalTest
)

$ErrorActionPreference = "Stop"

function Write-Step($m)  { Write-Host "==> $m" -ForegroundColor Cyan }
function Write-Ok($m)    { Write-Host "    $m" -ForegroundColor Green }
function Write-Note($m)  { Write-Host "    $m" -ForegroundColor DarkGray }

# ---------------------------------------------------------------- paths
$ScriptDir = $PSScriptRoot
$RepoRoot  = (Resolve-Path (Join-Path $ScriptDir "..\..")).Path
$IconsDir  = Join-Path $RepoRoot "src-tauri\icons"
$Master    = Join-Path $IconsDir "icon.png"
$Template  = Join-Path $ScriptDir "AppxManifest.xml"
$Staging   = Join-Path $ScriptDir "staging"
$Assets    = Join-Path $Staging "Assets"

if ([string]::IsNullOrWhiteSpace($ExePath)) {
    $ExePath = Join-Path $RepoRoot "target\release\voidgif.exe"
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $ScriptDir ("VoidGif_{0}_x64.msix" -f $Version)
}

# ---------------------------------------------------------------- preflight
Write-Step "Preflight"
if (-not (Test-Path $ExePath)) {
    throw "Release exe not found: $ExePath`n    Build it first:  cargo build --release --manifest-path src-tauri\Cargo.toml"
}
Write-Ok "release exe : $ExePath"
if (-not (Test-Path $Master)) { throw "Master icon not found: $Master" }
if (-not (Test-Path $Template)) { throw "Manifest template not found: $Template" }

# Locate the newest x64 SDK tool (makeappx / makepri / signtool).
function Find-SdkTool([string]$name) {
    $roots = @(
        "C:\Program Files (x86)\Windows Kits\10\bin",
        "C:\Program Files\Windows Kits\10\bin"
    )
    $hits = @()
    foreach ($r in $roots) {
        if (Test-Path $r) {
            $hits += Get-ChildItem $r -Recurse -Filter $name -ErrorAction SilentlyContinue |
                     Where-Object { $_.FullName -match '\\x64\\' }
        }
    }
    if ($hits.Count -eq 0) { return $null }
    # Highest SDK build number wins.
    ($hits | Sort-Object { $_.FullName } -Descending | Select-Object -First 1).FullName
}

$MakeAppx = Find-SdkTool "makeappx.exe"
$MakePri  = Find-SdkTool "makepri.exe"
if (-not $MakeAppx) { throw "makeappx.exe not found under any Windows 10 SDK. Install the Windows SDK." }
Write-Ok "makeappx    : $MakeAppx"
if ($MakePri)  { Write-Ok "makepri     : $MakePri" } else { Write-Note "makepri not found — resources.pri will be skipped." ; $SkipPri = $true }

# ---------------------------------------------------------------- manifest
Write-Step "Preparing manifest"
$manifest = Get-Content -Raw -Encoding UTF8 $Template

if (-not [string]::IsNullOrWhiteSpace($IdentityName))        { $manifest = $manifest.Replace("{{IDENTITY_NAME}}", $IdentityName) }
if (-not [string]::IsNullOrWhiteSpace($Publisher))           { $manifest = $manifest.Replace("{{PUBLISHER}}", $Publisher) }
if (-not [string]::IsNullOrWhiteSpace($PublisherDisplayName)){ $manifest = $manifest.Replace("{{PUBLISHER_DISPLAY_NAME}}", $PublisherDisplayName) }

# Apply a non-default version.
if ($Version -ne "0.1.0.0") {
    $manifest = [regex]::Replace($manifest, 'Version="0\.1\.0\.0"', ('Version="{0}"' -f $Version))
}

# Any placeholder left unfilled means the owner neither passed the parameter
# nor edited the template — stop before we build an invalid package.
$leftover = [regex]::Matches($manifest, '\{\{[A-Z_]+\}\}') | ForEach-Object { $_.Value } | Sort-Object -Unique
if ($leftover.Count -gt 0) {
    throw ("Unfilled manifest placeholders: {0}`n    Pass -IdentityName / -Publisher / -PublisherDisplayName, or edit store\msix\AppxManifest.xml." -f ($leftover -join ", "))
}
Write-Ok "identity values applied"

# ---------------------------------------------------------------- staging
Write-Step "Staging package tree"
if (Test-Path $Staging) { Remove-Item $Staging -Recurse -Force }
New-Item -ItemType Directory -Path $Assets -Force | Out-Null

Copy-Item $ExePath (Join-Path $Staging "VoidGif.exe") -Force
Write-Ok "staged VoidGif.exe"

# Write the finished manifest into the package root.
Set-Content -Path (Join-Path $Staging "AppxManifest.xml") -Value $manifest -Encoding UTF8
Write-Ok "staged AppxManifest.xml"

# ---------------------------------------------------------------- assets
Write-Step "Generating tile / logo assets from icon.png"
Add-Type -AssemblyName System.Drawing

$src = [System.Drawing.Image]::FromFile($Master)

function Save-Square([int]$size, [string]$file) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.DrawImage($src, (New-Object System.Drawing.Rectangle 0, 0, $size, $size))
    $g.Dispose()
    $bmp.Save((Join-Path $Assets $file), [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function Save-Wide([int]$w, [int]$h, [string]$file) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)
    # Center a square glyph (height-tall) on the wide canvas.
    $x = [int](($w - $h) / 2)
    $g.DrawImage($src, (New-Object System.Drawing.Rectangle $x, 0, $h, $h))
    $g.Dispose()
    $bmp.Save((Join-Path $Assets $file), [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

# Base assets referenced by AppxManifest.xml.
Save-Square 50  "StoreLogo.png"
Save-Square 44  "Square44x44Logo.png"
Save-Square 71  "Square71x71Logo.png"
Save-Square 150 "Square150x150Logo.png"
Save-Square 310 "Square310x310Logo.png"
Save-Wide   310 150 "Wide310x150Logo.png"
# Extra scale / unplated variants — resolved via resources.pri for crisper
# rendering; harmless if PRI is skipped (the base names are still used).
Save-Square 88  "Square44x44Logo.scale-200.png"
Save-Square 300 "Square150x150Logo.scale-200.png"
Save-Square 44  "Square44x44Logo.targetsize-44_altform-unplated.png"

$src.Dispose()
Write-Ok ("generated {0} PNG assets" -f (Get-ChildItem $Assets -Filter *.png).Count)

# ---------------------------------------------------------------- resources.pri
if (-not $SkipPri) {
    Write-Step "Building resources.pri"
    $priConfig = Join-Path $ScriptDir "priconfig.xml"
    try {
        & $MakePri createconfig /cf $priConfig /dq "en-US" /o | Out-Null
        # Index the staging tree; /pr is the package root that holds
        # AppxManifest.xml + Assets. Run from the staging dir so paths in the
        # generated PRI are package-relative.
        Push-Location $Staging
        & $MakePri new /pr $Staging /cf $priConfig /of (Join-Path $Staging "resources.pri") /o | Out-Null
        Pop-Location
        if (Test-Path (Join-Path $Staging "resources.pri")) {
            Write-Ok "resources.pri built"
        } else {
            Write-Warning "resources.pri was not produced; continuing without it."
        }
        Remove-Item $priConfig -Force -ErrorAction SilentlyContinue
    } catch {
        if ((Get-Location).Path -eq $Staging) { Pop-Location }
        Write-Warning ("makepri failed ({0}); continuing without resources.pri. Base-named assets still resolve." -f $_.Exception.Message)
    }
}

# ---------------------------------------------------------------- pack
Write-Step "Packing MSIX"
if (Test-Path $Output) { Remove-Item $Output -Force }
& $MakeAppx pack /d $Staging /p $Output /o
if ($LASTEXITCODE -ne 0) { throw "makeappx pack failed with exit code $LASTEXITCODE" }

$sizeKB = [int]((Get-Item $Output).Length / 1KB)
Write-Ok ("built {0} ({1} KB)" -f (Split-Path $Output -Leaf), $sizeKB)

# ---------------------------------------------------------------- optional local-test signing
if ($SelfSignForLocalTest) {
    Write-Step "Self-signing a LOCAL-TEST copy (do NOT upload this one)"
    $SignTool = Find-SdkTool "signtool.exe"
    if (-not $SignTool) { throw "signtool.exe not found; cannot self-sign." }

    # Subject must equal the manifest Publisher, or Windows refuses to install.
    $pubMatch = [regex]::Match($manifest, 'Publisher="([^"]+)"')
    $subject = $pubMatch.Groups[1].Value
    Write-Note "cert subject = $subject"

    $cert = New-SelfSignedCertificate -Type Custom -Subject $subject `
        -KeyUsage DigitalSignature -FriendlyName "VoidGif local test" `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")

    $pfx = Join-Path $ScriptDir "voidgif-localtest.pfx"
    $cer = Join-Path $ScriptDir "voidgif-localtest.cer"
    $pw = ConvertTo-SecureString -String "voidgif" -Force -AsPlainText
    Export-PfxCertificate -Cert ("Cert:\CurrentUser\My\" + $cert.Thumbprint) -FilePath $pfx -Password $pw | Out-Null
    Export-Certificate -Cert ("Cert:\CurrentUser\My\" + $cert.Thumbprint) -FilePath $cer | Out-Null

    $signed = [System.IO.Path]::ChangeExtension($Output, $null).TrimEnd('.') + "_signed.msix"
    Copy-Item $Output $signed -Force
    & $SignTool sign /fd SHA256 /a /f $pfx /p "voidgif" $signed
    if ($LASTEXITCODE -ne 0) { throw "signtool sign failed with exit code $LASTEXITCODE" }
    Write-Ok "signed local-test copy: $signed"
    Write-Note "To install locally: import $cer into 'Trusted People' (admin), then Add-AppxPackage $signed"
}

# ---------------------------------------------------------------- done
Write-Host ""
Write-Step "Done"
Write-Host "  Upload to Partner Center:  $Output" -ForegroundColor Green
Write-Note "This .msix is intentionally UNSIGNED — the Store signs it during certification."
