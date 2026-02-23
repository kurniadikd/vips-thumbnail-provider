# PowerShell Script to register the libvips Thumbnail Provider

$clsid = "{D3A2F1B2-7E8B-4C9D-A3D1-2F0B3C4D5E6F}"
$dllPath = Join-Path $PSScriptRoot "target\release\thumbnail_generator.dll"

if (-not (Test-Path $dllPath)) {
    Write-Error "DLL not found at $dllPath. Please run 'cargo build --release' first."
    exit
}

# 1. Register CLSID
$clsidPath = "HKCU:\Software\Classes\CLSID\$clsid"
$inprocPath = "$clsidPath\InprocServer32"

if (-not (Test-Path $clsidPath)) { New-Item -Path $clsidPath -Force }
Set-ItemProperty -Path $clsidPath -Name "(Default)" -Value "libvips Thumbnail Provider"
Set-ItemProperty -Path $clsidPath -Name "DisableCache" -Value 1 -Type DWord

if (-not (Test-Path $inprocPath)) { New-Item -Path $inprocPath -Force }
Set-ItemProperty -Path $inprocPath -Name "(Default)" -Value $dllPath
Set-ItemProperty -Path $inprocPath -Name "ThreadingModel" -Value "Both"

# 2. Associate file extensions (Comprehensive list supported by libvips)
$extensions = @(
    ".avif", ".heic", ".heif", ".webp", ".jxl", 
    ".jpg", ".jpeg", ".jpe", ".png", ".gif", 
    ".tif", ".tiff", ".svg", ".pdf", ".bmp",
    ".ico", ".dcm", ".dicom", ".jp2", ".j2k",
    ".svs", ".ndpi", ".vms", ".vmu", ".scn", ".mrxs",
    ".fits", ".mat", ".csv"
)
$providerInterface = "{E357FCCD-A995-4576-B01F-234630154E96}" # IThumbnailProvider interface ID

foreach ($ext in $extensions) {
    # Register under Extension
    $shellExPath = "HKCU:\Software\Classes\$ext\ShellEx\$providerInterface"
    if (-not (Test-Path $shellExPath)) { New-Item -Path $shellExPath -Force }
    Set-ItemProperty -Path $shellExPath -Name "(Default)" -Value $clsid
    
    # Force Windows to treat this as a "Thumbnail" type
    $extPath = "HKCU:\Software\Classes\$ext"
    Set-ItemProperty -Path $extPath -Name "Treatment" -Value 2 -ErrorAction SilentlyContinue
    
    # Register under associated ProgID (e.g., ChromeHTML, PhotoViewer.FileAssoc.Tiff)
    try {
        $progId = (Get-ItemProperty "HKCU:\Software\Classes\$ext" -ErrorAction SilentlyContinue)."(Default)"
        if (-not $progId) {
            $progId = (Get-ItemProperty "HKLM:\Software\Classes\$ext" -ErrorAction SilentlyContinue)."(Default)"
        }
        if ($progId) {
            $progIdPath = "HKCU:\Software\Classes\$progId\ShellEx\$providerInterface"
            if (-not (Test-Path $progIdPath)) { New-Item -Path $progIdPath -Force }
            Set-ItemProperty -Path $progIdPath -Name "(Default)" -Value $clsid
            Write-Host "Associated $ext ($progId) with libvips provider."
        } else {
            Write-Host "Associated $ext with libvips provider."
        }
    } catch {
        Write-Host "Associated $ext with libvips provider."
    }
}

# 3. Clear Thumbnail Cache
Write-Host "`nCleaning thumbnail cache..."
taskkill /F /IM explorer.exe 2>$null
# Clear cache files
Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\Windows\Explorer\thumbcache_*.db" | Remove-Item -Force -ErrorAction SilentlyContinue
start explorer.exe

Write-Host "`nRegistration complete! Thumbnail cache cleared and Explorer restarted."
Write-Host "Log file: C:\vips\thumb_log.txt (check this if thumbnails don't appear)"
