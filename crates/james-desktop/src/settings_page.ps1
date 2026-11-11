# Only a closed page identifier crosses this boundary. No caller-provided URI,
# arguments, executable or script text is used to launch a process.
$ErrorActionPreference = 'Stop'
try {
    $inputText = [Console]::In.ReadToEnd()
    if ($inputText.Length -gt 256) { throw 'Settings request is too large.' }
    $request = $inputText | ConvertFrom-Json
    if ($null -eq $request -or $request -isnot [pscustomobject]) {
        throw 'Expected a settings request object.'
    }
    $properties = @($request.PSObject.Properties.Name)
    if ($properties.Count -ne 1 -or $properties[0] -cne 'page' -or $request.page -isnot [string]) {
        throw 'Expected only a known settings page.'
    }
    # Exact case-sensitive identifiers, also checked by the Rust enum.
    $uri = switch -CaseSensitive ($request.page) {
        'display' { 'ms-settings:display'; break }
        'sound' { 'ms-settings:sound'; break }
        'microphone_privacy' { 'ms-settings:privacy-microphone'; break }
        'bluetooth' { 'ms-settings:bluetooth'; break }
        'keyboard_accessibility' { 'ms-settings:easeofaccess-keyboard'; break }
        'notifications' { 'ms-settings:notifications'; break }
        default { throw 'Unsupported settings page.' }
    }
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = $uri
    $start.UseShellExecute = $true
    # The person explicitly requested the visible Windows Settings surface.
    [void][System.Diagnostics.Process]::Start($start)
    @{ opened = $true; page = $request.page } | ConvertTo-Json -Compress
} catch {
    [Console]::Error.WriteLine('Windows could not open the reviewed settings page.')
    exit 1
}
