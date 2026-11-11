param(
  [Parameter(Mandatory=$true)][ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')][string]$Repository,
  [Parameter(Mandatory=$true)][string]$CurrentVersion,
  [switch]$Silent,
  [int]$ParentPid = 0
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$work = $null
$readyFile = Join-Path $env:LOCALAPPDATA 'LovingCoolRust\update.ready'
$statusFile = Join-Path $env:LOCALAPPDATA 'LovingCoolRust\update.status'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $statusFile) | Out-Null
function Set-Status([string]$message) {
  Set-Content -LiteralPath $statusFile -Value $message -Encoding UTF8
}
try {
  Remove-Item -LiteralPath $readyFile -Force -ErrorAction SilentlyContinue
  if ($ParentPid -le 0) { throw 'Invalid parent process ID' }
  Set-Status 'Đang kiểm tra GitHub Releases...'
  $headers = @{ 'User-Agent' = 'LovingCoolRust-Updater'; 'Accept' = 'application/vnd.github+json' }
  $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repository/releases/latest" -Headers $headers -TimeoutSec 15
  $remote = [Version]($release.tag_name.TrimStart('v'))
  $installed = [Version]($CurrentVersion.TrimStart('v'))
  if ($remote -le $installed) {
    Set-Status "Bạn đang dùng bản mới nhất (v$CurrentVersion)."
    exit 0
  }
  $zipAsset = @($release.assets) | Where-Object { $_.name -eq 'lovingcool-dp01-windows-x64.zip' } | Select-Object -First 1
  $shaAsset = @($release.assets) | Where-Object { $_.name -eq 'lovingcool-dp01-windows-x64.sha256' } | Select-Object -First 1
  if (!$zipAsset -or !$shaAsset) { throw 'Release thiếu ZIP hoặc checksum' }

  Set-Status "Đang tải v$remote..."
  $work = Join-Path $env:TEMP ('lovingcool-update-' + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Path $work | Out-Null
  $archive = Join-Path $work 'release.zip'
  $checksum = Join-Path $work 'release.sha256'
  Invoke-WebRequest -Uri $zipAsset.browser_download_url -Headers $headers -OutFile $archive -TimeoutSec 120
  Invoke-WebRequest -Uri $shaAsset.browser_download_url -Headers $headers -OutFile $checksum -TimeoutSec 30

  Set-Status 'Đang xác minh gói cập nhật (SHA-256)...'
  $expected = ((Get-Content $checksum -Raw).Trim() -split '\s+')[0].ToLowerInvariant()
  $actual = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
  if ($expected -notmatch '^[0-9a-f]{64}$' -or $actual -ne $expected) { throw 'Checksum SHA-256 không khớp' }
  $unpacked = Join-Path $work 'unpacked'
  Expand-Archive -Path $archive -DestinationPath $unpacked -Force
  $newExe = Join-Path $unpacked 'lovingcool-dp01-studio.exe'
  $newUpdater = Join-Path $unpacked 'update.ps1'
  if (!(Test-Path -LiteralPath $newExe) -or !(Test-Path -LiteralPath $newUpdater)) { throw 'Bản phát hành thiếu EXE hoặc updater' }

  # Notify only after download, integrity verification and extraction succeeded.
  Set-Status 'Đã tải xong. Đang khởi động lại ứng dụng...'
  Set-Content -LiteralPath $readyFile -Value $release.tag_name -Encoding UTF8
  try { Wait-Process -Id $ParentPid -Timeout 90 -ErrorAction Stop }
  catch { throw 'Ứng dụng chưa đóng; đã hủy cập nhật' }
  $target = Split-Path -Parent $MyInvocation.MyCommand.Path
  Copy-Item -LiteralPath $newExe -Destination (Join-Path $target 'lovingcool-dp01-studio.exe') -Force
  Copy-Item -LiteralPath $newUpdater -Destination (Join-Path $target 'update.ps1') -Force
  Set-Status "Đã cài đặt v$remote."
  Start-Process -FilePath (Join-Path $target 'lovingcool-dp01-studio.exe')
} catch {
  Set-Status ("Cập nhật thất bại: " + $_.Exception.Message)
} finally {
  Remove-Item -LiteralPath $readyFile -Force -ErrorAction SilentlyContinue
  if ($work) { Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue }
}
