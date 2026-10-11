$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    Add-Type -AssemblyName System.Speech
    $recognizer = [Speech.Recognition.SpeechRecognitionEngine]::new()
    try {
        $recognizer.LoadGrammar([Speech.Recognition.DictationGrammar]::new())
        $recognizer.SetInputToDefaultAudioDevice()
        $recognizer.InitialSilenceTimeout = [TimeSpan]::FromSeconds($request.seconds)
        $recognizer.BabbleTimeout = [TimeSpan]::FromSeconds($request.seconds)
        $result = $recognizer.Recognize([TimeSpan]::FromSeconds($request.seconds))
        $text = if ($null -eq $result) { '' } else { $result.Text }
        [Console]::Write((ConvertTo-Json -InputObject $text -Compress))
    } finally { $recognizer.Dispose() }
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
