[CmdletBinding()]
param(
    [string]$IsccPath = '',
    [ValidateSet('x86_64-pc-windows-msvc', 'x86_64-pc-windows-gnu')]
    [string]$Target = 'x86_64-pc-windows-msvc',
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:OS -ne 'Windows_NT') { throw 'The James installer must be built on Windows.' }
$sourceRoot = Split-Path -Parent $PSScriptRoot

if (-not $IsccPath) {
    $compiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($compiler) { $IsccPath = $compiler.Source }
    else { $IsccPath = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe' }
}
if (-not (Test-Path -LiteralPath $IsccPath -PathType Leaf)) {
    throw 'Install Inno Setup 6 or supply -IsccPath with the full path to ISCC.exe.'
}
$IsccPath = (Resolve-Path -LiteralPath $IsccPath).Path

Push-Location $sourceRoot
try {
    $metadataText = & cargo metadata --no-deps --format-version 1 --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'Unable to read Cargo workspace metadata.' }
    $metadata = ($metadataText -join "`n") | ConvertFrom-Json
    $package = @($metadata.packages | Where-Object { $_.name -eq 'james-desktop' })
    if ($package.Count -ne 1) { throw 'Expected exactly one james-desktop workspace package.' }
    $version = $package[0].version
    if ($version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$') {
        throw "Unsupported package version: $version"
    }
    # Reject release tags that could mislabel an installer built from a different version.
    if ($env:GITHUB_REF -like 'refs/tags/v*' -and $env:GITHUB_REF -ne "refs/tags/v$version") {
        throw "Release tag does not match the Cargo package version $version."
    }
    if (-not $SkipBuild) {
        & cargo build -p james-desktop --release --locked --target $target
        if ($LASTEXITCODE -ne 0) { throw 'James release build failed.' }
    }
    $binary = Join-Path $metadata.target_directory "$target\release\james.exe"
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "Release binary missing: $binary. Build for $target first."
    }
    $outputPath = Join-Path $sourceRoot 'target\installer'
    New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
    & $IsccPath "/DAppVersion=$version" "/DSourceRoot=$sourceRoot" "/DAppBinary=$binary" "/DOutputPath=$outputPath" (Join-Path $sourceRoot 'installer\james.iss')
    if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed.' }
    $installerPath = Join-Path $outputPath "James-$version-windows-x64-setup.exe"
    if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) { throw 'Installer output missing.' }
    $checksum = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    "$checksum  $([System.IO.Path]::GetFileName($installerPath))" |
        Set-Content -LiteralPath "$installerPath.sha256" -Encoding ascii
    Write-Output "Installer: $installerPath"
    Write-Output "SHA256: $checksum"
}
finally {
    Pop-Location
}
