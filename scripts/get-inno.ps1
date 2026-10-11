[CmdletBinding()]
param(
    [string]$DestinationPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'target/tools/inno-6.4.3')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:OS -ne 'Windows_NT') { throw 'The Inno Setup compiler requires Windows.' }
$expectedHash = '9d67a2b1155ca5bc825475ceb907aa357c5feb42e474cfd5347eddeef5ad92a5'
$expectedSigner = 'E0AB19C8D38CBF9C44709925122A7A02F8C70CB7'
$uri = 'https://api.nuget.org/v3-flatcontainer/tools.innosetup/6.4.3/tools.innosetup.6.4.3.nupkg'
New-Item -ItemType Directory -Path $DestinationPath -Force | Out-Null
$DestinationPath = (Resolve-Path -LiteralPath $DestinationPath).Path
$archive = Join-Path $DestinationPath 'compiler.zip'
Invoke-WebRequest -Uri $uri -OutFile $archive -TimeoutSec 120
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expectedHash) {
    throw 'Inno Setup package checksum mismatch; refusing to extract or run it.'
}
Expand-Archive -LiteralPath $archive -DestinationPath $DestinationPath -Force
$compiler = Join-Path $DestinationPath 'tools/ISCC.exe'
$signature = Get-AuthenticodeSignature -LiteralPath $compiler
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $expectedSigner) {
    throw 'Inno Setup compiler signature does not match the trusted publisher.'
}
Write-Output $compiler
