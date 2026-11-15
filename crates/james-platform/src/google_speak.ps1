$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    # The API key arrives on stdin with the request and is used only for this
    # one call; it never appears on a command line or in an error message.
    $headers = @{ 'X-Goog-Api-Key' = [string]$request.key }
    $voice = @{ languageCode = [string]$request.language }
    if ($request.voice) { $voice['name'] = [string]$request.voice }
    $body = @{
        input = @{ text = [string]$request.text }
        voice = $voice
        audioConfig = @{ audioEncoding = 'LINEAR16'; sampleRateHertz = 24000 }
    } | ConvertTo-Json -Depth 5
    $response = Invoke-RestMethod -Uri 'https://texttospeech.googleapis.com/v1/text/synthesize' -Method Post -ContentType 'application/json' -Headers $headers -Body $body -TimeoutSec 90
    $pcm = [Convert]::FromBase64String([string]$response.audioContent)
    if ($null -eq $pcm -or $pcm.Length -eq 0) { throw 'Google returned no audio.' }
    # LINEAR16 is raw 16-bit mono PCM, which SoundPlayer cannot play directly:
    # wrap it in a 44-byte WAV header first.
    $rate = 24000
    $channels = 1
    $bits = 16
    $data = $pcm.Length
    $riff = [System.IO.MemoryStream]::new()
    $writer = [System.IO.BinaryWriter]::new($riff)
    $writer.Write([byte[]]@(0x52, 0x49, 0x46, 0x46))                # 'RIFF'
    $writer.Write([uint32](36 + $data))                            # rest of the file
    $writer.Write([byte[]]@(0x57, 0x41, 0x56, 0x45))              # 'WAVE'
    $writer.Write([byte[]]@(0x66, 0x6D, 0x74, 0x20))                # 'fmt '
    $writer.Write([uint32]16)                                      # PCM chunk size
    $writer.Write([uint16]1)                                       # PCM format
    $writer.Write([uint16]$channels)
    $writer.Write([uint32]$rate)
    $writer.Write([uint32]($rate * $channels * $bits / 8))         # byte rate
    $writer.Write([uint16]($channels * $bits / 8))                # block align
    $writer.Write([uint16]$bits)
    $writer.Write([byte[]]@(0x64, 0x61, 0x74, 0x61))                # 'data'
    $writer.Write([uint32]$data)
    $writer.Write($pcm)
    $writer.Flush()
    $player = [System.Media.SoundPlayer]::new($riff)
    $player.PlaySync()
    $player.Dispose()
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
