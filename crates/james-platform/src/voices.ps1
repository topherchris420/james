$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    Add-Type -AssemblyName System.Speech
    $speaker = [Speech.Synthesis.SpeechSynthesizer]::new()
    try {
        # Build the JSON array by hand: ConvertTo-Json unwraps a one-item
        # array in Windows PowerShell 5.1, which would break the parser.
        $items = @($speaker.GetInstalledVoices() |
            Where-Object { $_.Enabled } |
            ForEach-Object { ConvertTo-Json -InputObject $_.VoiceInfo.Name })
    }
    finally { $speaker.Dispose() }
    [Console]::Out.Write("[" + ($items -join ",") + "]")
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
