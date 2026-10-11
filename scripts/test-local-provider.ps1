# Check James's local inference connection without changing LM Studio configuration.
# Run: pwsh -File scripts/test-local-provider.ps1 -Provider LMStudio
# Then optionally pass -Model '<the exact ID printed by the first command>'.
[CmdletBinding()]
param(
    [ValidateSet('LMStudio', 'Ollama', 'LlamaCpp', 'OpenAICompatible')]
    [string]$Provider = 'LMStudio',
    [ValidateRange(1, 65535)]
    [int]$Port = 1234,
    [string]$Model = ''
)

$ErrorActionPreference = 'Stop'
if (-not $PSBoundParameters.ContainsKey('Port')) {
    $Port = switch ($Provider) {
        'Ollama' { 11434 }
        'LlamaCpp' { 8080 }
        default { 1234 }
    }
}
$BaseUrl = "http://127.0.0.1:$Port"
$NativeOllama = $Provider -eq 'Ollama'
$ListPath = if ($NativeOllama) { '/api/tags' } else { '/v1/models' }
$ChatPath = if ($NativeOllama) { '/api/chat' } else { '/v1/chat/completions' }

Write-Host "Probing $Provider at $BaseUrl (loopback only)"
try {
    $Listing = Invoke-RestMethod -Method Get -Uri "$BaseUrl$ListPath" -TimeoutSec 8
    $Identifiers = if ($NativeOllama) {
        @($Listing.models | ForEach-Object { $_.name })
    } else {
        @($Listing.data | ForEach-Object { $_.id })
    }
    $Identifiers = @($Identifiers | Where-Object { $_ })
    if ($Identifiers.Count -eq 0) {
        throw 'Server responded, but did not list any models. Load a model before continuing.'
    }
    Write-Host "Server reachable. Model IDs:"
    $Identifiers | ForEach-Object { Write-Host "  $_" }

    if (-not $Model) {
        Write-Host 'Model discovery passed. To test real inference, rerun with -Model and one of the exact IDs above.'
        exit 0
    }
    if ($Model -notin $Identifiers) {
        throw "Requested model '$Model' is not listed by the server."
    }

    $Payload = @{
        model = $Model
        stream = $false
        messages = @(@{ role = 'user'; content = 'Reply with the word READY.' })
    } | ConvertTo-Json -Depth 8 -Compress
    $Elapsed = [System.Diagnostics.Stopwatch]::StartNew()
    $Result = Invoke-RestMethod -Method Post -Uri "$BaseUrl$ChatPath" -ContentType 'application/json' -Body $Payload -TimeoutSec 120
    $Elapsed.Stop()
    $Answer = if ($NativeOllama) { $Result.message.content } else { $Result.choices[0].message.content }
    if ([string]::IsNullOrWhiteSpace($Answer)) {
        throw 'The response did not contain nonempty assistant text.'
    }
    Write-Host ("Real text inference succeeded in {0:N2} seconds." -f $Elapsed.Elapsed.TotalSeconds)
    Write-Host "Assistant reply: $Answer"
    Write-Host 'This does not validate vision, tool calls, streaming, cancellation or accessibility.'
} catch {
    Write-Error "Local inference check failed: $($_.Exception.Message)"
    exit 1
}
