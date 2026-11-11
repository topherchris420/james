$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $managerType = [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager,Windows.Media.Control,ContentType=WindowsRuntime]
    $propertiesType = [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionMediaProperties,Windows.Media.Control,ContentType=WindowsRuntime]
    $asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.IsGenericMethodDefinition -and $_.GetGenericArguments().Count -eq 1 -and
        $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
    } | Select-Object -First 1
    if ($null -eq $asTask) { throw 'Windows media async support is unavailable.' }
    function Await-WinRT($operation, [type]$resultType) {
        $task = $asTask.MakeGenericMethod($resultType).Invoke($null, @($operation))
        if (-not $task.Wait(5000)) { throw 'Windows media request timed out.' }
        return $task.Result
    }
    $manager = Await-WinRT ($managerType::RequestAsync()) $managerType
    $session = $manager.GetCurrentSession()
    if ($request.action -eq 'query') {
        if ($null -eq $session) { [Console]::Write('{"active":false}'); exit 0 }
        $properties = Await-WinRT ($session.TryGetMediaPropertiesAsync()) $propertiesType
        $info = $session.GetPlaybackInfo()
        $controls = $info.Controls
        $state = [string]$info.PlaybackStatus
        $canToggle = [bool]$controls.IsPlayPauseToggleEnabled -or (($state -eq 'Playing') -and [bool]$controls.IsPauseEnabled) -or (($state -ne 'Playing') -and [bool]$controls.IsPlayEnabled)
        $data = @{ active=$true; title=[string]$properties.Title; artist=[string]$properties.Artist; source=[string]$session.SourceAppUserModelId; playing=($state -eq 'Playing'); can_toggle=$canToggle; can_next=[bool]$controls.IsNextEnabled }
        [Console]::Write(($data | ConvertTo-Json -Compress))
        exit 0
    }
    if ($request.action -notin @('toggle','next')) { throw 'Unknown media control.' }
    if ($null -eq $session -or [string]::IsNullOrEmpty([string]$request.source) -or [string]$session.SourceAppUserModelId -cne [string]$request.source) {
        throw 'The current media source changed. Refresh before controlling playback.'
    }
    $info = $session.GetPlaybackInfo()
    $controls = $info.Controls
    if ($request.action -eq 'next') {
        if (-not $controls.IsNextEnabled) { throw 'The player does not support next.' }
        $operation = $session.TrySkipNextAsync()
    } elseif ($controls.IsPlayPauseToggleEnabled) {
        $operation = $session.TryTogglePlayPauseAsync()
    } elseif ([string]$info.PlaybackStatus -eq 'Playing' -and $controls.IsPauseEnabled) {
        $operation = $session.TryPauseAsync()
    } elseif ([string]$info.PlaybackStatus -ne 'Playing' -and $controls.IsPlayEnabled) {
        $operation = $session.TryPlayAsync()
    } else { throw 'The player does not support play or pause.' }
    $accepted = [bool](Await-WinRT $operation ([bool]))
    [Console]::Write((@{ok=$accepted} | ConvertTo-Json -Compress))
} catch { [Console]::Error.Write($_.Exception.Message); exit 1 }
