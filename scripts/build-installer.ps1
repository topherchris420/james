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

# Signs a file with Authenticode only when a maintainer provides real
# credentials, either:
# - Azure Artifact Signing: ARTIFACT_SIGNING_ACCOUNT, ARTIFACT_SIGNING_ENDPOINT
#   and ARTIFACT_SIGNING_PROFILE, with ARTIFACT_SIGNING_DLIB (the path that
#   scripts/get-artifact-signing.ps1 prints). It signs as the Azure identity in
#   AZURE_TENANT_ID, AZURE_CLIENT_ID and AZURE_CLIENT_SECRET, or as the
#   signed-in Azure CLI user; or
# - a certificate file: WINDOWS_CERTIFICATE_PFX (path to a .pfx file) and
#   WINDOWS_CERTIFICATE_PASSWORD.
# WINDOWS_TIMESTAMP_URL overrides the timestamp server. Without credentials
# the build stays unsigned and says so; signing is never faked.
function Invoke-CodeSign([string]$Path) {
    $artifactSigning = [bool]$env:ARTIFACT_SIGNING_ACCOUNT
    if (-not $artifactSigning -and -not $env:WINDOWS_CERTIFICATE_PFX) { return $false }
    $signtool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $signtool) { throw 'Code signing is configured, but signtool.exe was not found.' }
    if ($artifactSigning) {
        foreach ($name in 'ARTIFACT_SIGNING_DLIB', 'ARTIFACT_SIGNING_ENDPOINT', 'ARTIFACT_SIGNING_PROFILE') {
            if (-not [Environment]::GetEnvironmentVariable($name)) { throw "ARTIFACT_SIGNING_ACCOUNT is set, but $name is not." }
        }
        $timestamp = if ($env:WINDOWS_TIMESTAMP_URL) { $env:WINDOWS_TIMESTAMP_URL } else { 'http://timestamp.acs.microsoft.com' }
        $metadata = Join-Path ([IO.Path]::GetTempPath()) "james-artifact-signing-$PID.json"
        $settings = [ordered]@{
            Endpoint = $env:ARTIFACT_SIGNING_ENDPOINT
            CodeSigningAccountName = $env:ARTIFACT_SIGNING_ACCOUNT
            CertificateProfileName = $env:ARTIFACT_SIGNING_PROFILE
        }
        # Without a byte-order mark, which Windows PowerShell's utf8 would add.
        [IO.File]::WriteAllText($metadata, ($settings | ConvertTo-Json))
        try {
            & $signtool.FullName sign /fd SHA256 /td SHA256 /tr $timestamp /dlib $env:ARTIFACT_SIGNING_DLIB /dmdf $metadata $Path | Out-Host
        }
        finally { Remove-Item -LiteralPath $metadata -Force }
    }
    else {
        $timestamp = if ($env:WINDOWS_TIMESTAMP_URL) { $env:WINDOWS_TIMESTAMP_URL } else { 'http://timestamp.digicert.com' }
        & $signtool.FullName sign /fd SHA256 /td SHA256 /tr $timestamp /f $env:WINDOWS_CERTIFICATE_PFX /p $env:WINDOWS_CERTIFICATE_PASSWORD $Path | Out-Host
    }
    if ($LASTEXITCODE -ne 0) { throw "Code signing failed for $Path" }
    & $signtool.FullName verify /pa $Path | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Signature verification failed for $Path" }
    return $true
}

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
    $signed = Invoke-CodeSign $binary
    $outputPath = Join-Path $sourceRoot 'target\installer'
    New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
    # Licenses of the third-party crates compiled into james.exe: installed
    # with the app and published beside the portable copy.
    $notices = Join-Path $outputPath "James-$version-windows-x64-third-party-notices.txt"
    & cargo run --quiet --locked -p james-notices -- --target $target --output $notices
    if ($LASTEXITCODE -ne 0) { throw 'Could not write the third-party notices.' }
    if (-not (Select-String -LiteralPath $notices -Pattern '^eframe ' -Quiet)) { throw 'Third-party notices look incomplete.' }
    $noticesChecksum = (Get-FileHash -LiteralPath $notices -Algorithm SHA256).Hash.ToLowerInvariant()
    "$noticesChecksum  $([System.IO.Path]::GetFileName($notices))" |
        Set-Content -LiteralPath "$notices.sha256" -Encoding ascii
    & $IsccPath "/DAppVersion=$version" "/DSourceRoot=$sourceRoot" "/DAppBinary=$binary" "/DOutputPath=$outputPath" "/DNotices=$notices" (Join-Path $sourceRoot 'installer\james.iss')
    if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed.' }
    $installerPath = Join-Path $outputPath "James-$version-windows-x64-setup.exe"
    if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) { throw 'Installer output missing.' }
    if ($signed) { [void](Invoke-CodeSign $installerPath) }
    Write-Output $(if (-not $signed) { 'Not code-signed: no certificate configured.' }
        elseif ($env:ARTIFACT_SIGNING_ACCOUNT) { 'Signed with Azure Artifact Signing.' }
        else { 'Signed with the configured Authenticode certificate.' })
    $checksum = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    "$checksum  $([System.IO.Path]::GetFileName($installerPath))" |
        Set-Content -LiteralPath "$installerPath.sha256" -Encoding ascii
    Write-Output "Installer: $installerPath"
    Write-Output "SHA256: $checksum"
    # The same executable, named so a downloaded copy says what it is.
    $portablePath = Join-Path $outputPath "James-$version-windows-x64-portable.exe"
    Copy-Item -LiteralPath $binary -Destination $portablePath -Force
    $portableChecksum = (Get-FileHash -LiteralPath $portablePath -Algorithm SHA256).Hash.ToLowerInvariant()
    "$portableChecksum  $([System.IO.Path]::GetFileName($portablePath))" |
        Set-Content -LiteralPath "$portablePath.sha256" -Encoding ascii
    Write-Output "Portable: $portablePath"
}
finally {
    Pop-Location
}
