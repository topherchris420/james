$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;
public static class JamesCaptureDpi {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
    [void][JamesCaptureDpi]::SetProcessDPIAware()
    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -AssemblyName System.Drawing
    $bounds = [Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = [Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $memory = [IO.MemoryStream]::new()
    try {
        $graphics.CopyFromScreen($bounds.Location, [Drawing.Point]::Empty, $bounds.Size)
        $bitmap.Save($memory, [Drawing.Imaging.ImageFormat]::Png)
        $result = @{width=$bounds.Width; height=$bounds.Height; png=[Convert]::ToBase64String($memory.ToArray())}
        [Console]::Write(($result | ConvertTo-Json -Compress))
    } finally { $memory.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
