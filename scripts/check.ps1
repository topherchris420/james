param([string]$RustBin = '', [string]$LinkerDirectory = '')
$ErrorActionPreference = 'Stop'
if ($RustBin) {
    $resolvedRustBin = (Resolve-Path -LiteralPath $RustBin).Path
    $env:PATH = "$resolvedRustBin;$env:PATH"
    $env:RUSTC = Join-Path $resolvedRustBin 'rustc.exe'
}
if ($LinkerDirectory) {
    $resolvedLinker = (Resolve-Path -LiteralPath $LinkerDirectory).Path
    $env:PATH = "$resolvedLinker;$env:PATH"
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = 'x86_64-w64-mingw32-gcc.exe'
    $env:RUSTFLAGS = '-C link-self-contained=yes'
}
foreach ($cargoArguments in @(
    @('fmt', '--all', '--', '--check'),
    @('test', '--workspace', '--all-targets', '--locked', '--offline'),
    @('clippy', '--workspace', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings'),
    @('run', '--locked', '--offline', '--example', 'approval_demo')
)) {
    & cargo @cargoArguments
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
