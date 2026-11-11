param([string]$Repo = 'lovingcool-dp01-rust', [switch]$Private)
$ErrorActionPreference = 'Stop'
if (!(Get-Command gh -ErrorAction SilentlyContinue)) { throw 'Cài GitHub CLI từ https://cli.github.com/ rồi chạy gh auth login' }
if (!(Get-Command git -ErrorAction SilentlyContinue)) { throw 'Cài Git for Windows trước' }
if (!(Test-Path '.git')) { git init -b main }
git add .
git commit -m 'Initial Rust port with lightweight preview and GitHub updater'
$visibility = if ($Private) { '--private' } else { '--public' }
gh repo create $Repo $visibility --source . --remote origin --push
Write-Host "Repository created. Để phát hành v0.2.0: git tag v0.2.0; git push origin v0.2.0"
