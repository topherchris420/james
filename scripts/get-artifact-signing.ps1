[CmdletBinding()]
param(
    [string]$DestinationPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'target/tools/artifact-signing-1.0.128')
)

# Downloads the pinned SignTool plug-in for Azure Artifact Signing
# (Microsoft.ArtifactSigning.Client) and prints the path of its x64 dlib.
# It needs the .NET 8 runtime, which GitHub's Windows runners include.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:OS -ne 'Windows_NT') { throw 'Artifact Signing requires Windows.' }
$expectedHash = '74BD7D27E6CE1051409C38D9B46BC8DF0400ECD643D51FFBF2AC00869061E40B'
$expectedSigner = '3F56A45111684D454E231CFDC4DA5C8D370F9816'
$uri = 'https://api.nuget.org/v3-flatcontainer/microsoft.artifactsigning.client/1.0.128/microsoft.artifactsigning.client.1.0.128.nupkg'
New-Item -ItemType Directory -Path $DestinationPath -Force | Out-Null
$DestinationPath = (Resolve-Path -LiteralPath $DestinationPath).Path
$archive = Join-Path $DestinationPath 'client.zip'
Invoke-WebRequest -Uri $uri -OutFile $archive -TimeoutSec 120
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expectedHash) {
    throw 'Artifact Signing package checksum mismatch; refusing to extract or load it.'
}
Expand-Archive -LiteralPath $archive -DestinationPath $DestinationPath -Force
$dlib = Join-Path $DestinationPath 'bin/x64/Azure.CodeSigning.Dlib.dll'
$signature = Get-AuthenticodeSignature -LiteralPath $dlib
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $expectedSigner) {
    throw 'Artifact Signing dlib signature does not match the trusted publisher.'
}
Write-Output $dlib
