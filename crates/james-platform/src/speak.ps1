$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    Add-Type -AssemblyName System.Speech
    $speaker = [Speech.Synthesis.SpeechSynthesizer]::new()
    try {
        if ($request.voice) {
            # An unknown or removed voice falls back to the system default.
            try { $speaker.SelectVoice([string]$request.voice) } catch { }
        }
        $speaker.Speak([string]$request.text)
    }
    finally { $speaker.Dispose() }
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
