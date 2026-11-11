param(
  [Parameter(Mandatory=$true)][ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')][string]$Repository,
  [Parameter(Mandatory=$true)][string]$CurrentVersion,
  [switch]$Silent,
  [int]$ParentPid = 0
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$work = $null
try {
  if ($ParentPid -eq 0) { throw 'ParentPid required' }
  $headers = @{ 'User-Agent' = 'LovingCoolRust-Updater'; 'Accept' = 'application/vnd.github+json' }
  $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repository/releases/latest" -Headers $headers -TimeoutSec 15
  $remote = [Version]($release.tag_name.TrimStart('v'))
  $installed = [Version]($CurrentVersion.TrimStart('v'))
  if ($remote -le $installed) { exit 0 }
  $zipAsset = @($release.assets) | Where-Object { $_.name -eq 'lovingcool-dp01-windows-x64.zip' } | Select-Object -First 1
  $shaAsset = @($release.assets) | Where-Object { $_.name -eq 'lovingcool-dp01-windows-x64.sha256' } | Select-Object -First 1
  if (!$zipAsset -or !$shaAsset) { throw 'Release missing ZIP or checksum' }
  $work = Join-Path $env:TEMP ('lovingcool-update-' + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Path $work | Out-Null
  $archive = Join-Path $work 'release.zip'
  $checksum = Join-Path $work 'release.sha256'
  Invoke-WebRequest -Uri $zipAsset.browser_download_url -Headers $headers -OutFile $archive -TimeoutSec 90
  Invoke-WebRequest -Uri $shaAsset.browser_download_url -Headers $headers -OutFile $checksum -TimeoutSec 30
  $expected = ((Get-Content $checksum -Raw).Trim() -split '\s+')[0].ToLowerInvariant()
  $actual = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
  if ($expected -notmatch '^[0-9a-f]{64}$' -or $actual -ne $expected) { throw 'SHA-256 mismatch' }
  $unpacked = Join-Path $work 'unpacked'
  Expand-Archive -Path $archive -DestinationPath $unpacked -Force
  $newExe = Join-Path $unpacked 'lovingcool-dp01-studio.exe'
  if (!(Test-Path $newExe)) { throw 'Release ZIP does not contain expected EXE' }
  $target = Split-Path -Parent $MyInvocation.MyCommand.Path
  $readyDir = Join-Path $env:LOCALAPPDATA 'LovingCoolRust'
  New-Item -ItemType Directory -Path $readyDir -Force | Out-Null
  Set-Content -Path (Join-Path $readyDir 'update.ready') -Value $release.tag_name -Encoding UTF8
  # Running app notices the flag and exits, so no idle updater stays resident.
  try { Wait-Process -Id $ParentPid -Timeout 90 -ErrorAction Stop } catch { throw 'App did not exit in time; update cancelled' }
  Copy-Item -LiteralPath $newExe -Destination (Join-Path $target 'lovingcool-dp01-studio.exe') -Force
  Copy-Item -LiteralPath (Join-Path $unpacked 'update.ps1') -Destination (Join-Path $target 'update.ps1') -Force
  Start-Process -FilePath (Join-Path $target 'lovingcool-dp01-studio.exe')
} catch {
  if (!$Silent) { Write-Warning $_.Exception.Message }
} finally {
  if ($work) { Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue }
  if ($env:LOCALAPPDATA) { Remove-Item -LiteralPath (Join-Path $env:LOCALAPPDATA 'LovingCoolRust\update.ready') -Force -ErrorAction SilentlyContinue }
}
