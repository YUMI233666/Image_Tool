@echo off
setlocal
pushd "%~dp0"

if not defined REPO set "REPO=nihui/realcugan-ncnn-vulkan"
set "TARGET_DIR=src-tauri\binaries\models-se"

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$ErrorActionPreference = 'Stop';" ^
  "$repo = $env:REPO; if (-not $repo) { $repo = 'nihui/realcugan-ncnn-vulkan' }" ^
  "$api = \"https://api.github.com/repos/$repo/releases/latest\";" ^
  "$release = Invoke-RestMethod -Uri $api;" ^
  "$asset = $release.assets | Where-Object { $_.name -match 'models-se' } | Sort-Object { if ($_.name -match '\\.zip$') {0} elseif ($_.name -match '\\.7z$') {1} else {2} } | Select-Object -First 1;" ^
  "if (-not $asset) { throw \"No models-se asset found in $repo.\" }" ^
  "$archive = Join-Path $env:TEMP $asset.name;" ^
  "Write-Host \"Downloading $($asset.name)...\";" ^
  "Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $archive;" ^
  "$targetDir = Join-Path (Get-Location) '%TARGET_DIR%';" ^
  "New-Item -ItemType Directory -Force -Path $targetDir | Out-Null;" ^
  "if ($asset.name -match '\\.zip$') { Expand-Archive -Path $archive -DestinationPath $targetDir -Force } elseif ($asset.name -match '\\.7z$') { if (-not (Get-Command 7z -ErrorAction SilentlyContinue)) { throw '7z not found; install 7-Zip to extract .7z.' } 7z x $archive -o$targetDir -y | Out-Null } else { throw \"Unsupported archive type: $($asset.name)\" }" ^
  "Remove-Item $archive -Force;" ^
  "$nested = Join-Path $targetDir 'models-se'; if (Test-Path $nested) { Get-ChildItem -Path $nested -Force | Move-Item -Destination $targetDir -Force; Remove-Item -Path $nested -Recurse -Force }" ^
  "Write-Host \"Done: $targetDir\";"

set "EXITCODE=%ERRORLEVEL%"
popd
exit /b %EXITCODE%
