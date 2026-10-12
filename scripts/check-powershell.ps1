# Parses every PowerShell helper James embeds and compiles their C# interop
# declarations, so a syntax error fails CI instead of a person's first
# capture or approved action. Runs on Windows, and with pwsh on macOS/Linux.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$scripts = Get-ChildItem -Path (Join-Path $root 'crates/james-platform/src') -Filter '*.ps1'
$failed = $false
foreach ($script in $scripts) {
    $tokens = $null
    $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($script.FullName, [ref]$tokens, [ref]$errors)
    if ($errors.Count -gt 0) {
        $failed = $true
        foreach ($e in $errors) { Write-Host "$($script.Name):$($e.Extent.StartLineNumber): $($e.Message)" }
        continue
    }
    # Compile each embedded C# type definition without calling into it.
    $scriptFailed = $false
    $here = $ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.StringConstantExpressionAst] -and $n.StringConstantType -eq 'SingleQuotedHereString' }, $true)
    foreach ($source in $here) {
        if ($source.Value -notmatch 'DllImport') { continue }
        try {
            Add-Type -TypeDefinition $source.Value -ErrorAction Stop
        } catch {
            $scriptFailed = $true
            Write-Host "$($script.Name): embedded C# does not compile: $($_.Exception.Message)"
        }
    }
    if ($scriptFailed) { $failed = $true } else { Write-Host "$($script.Name): ok" }
}
if ($failed) { exit 1 }
