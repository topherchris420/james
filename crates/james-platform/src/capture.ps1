$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class JamesCapture {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
}
'@
    [void][JamesCapture]::SetProcessDPIAware()
    # The window in front while James is minimized is the one the person chose
    # to show. Naming it is best effort: a failure leaves the target empty and
    # never stops the capture.
    $target = $null
    try {
        $window = [JamesCapture]::GetForegroundWindow()
        if ($window -ne [IntPtr]::Zero) {
            $procId = [uint32]0
            [void][JamesCapture]::GetWindowThreadProcessId($window, [ref]$procId)
            $title = [Text.StringBuilder]::new(512)
            [void][JamesCapture]::GetWindowText($window, $title, 512)
            $name = ''
            $process = Get-Process -Id $procId -ErrorAction SilentlyContinue
            if ($null -ne $process) { $name = $process.ProcessName }
            $target = @{window=$window.ToInt64(); process_id=$procId; process_name=$name; title=$title.ToString()}
        }
    } catch { $target = $null }
    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -AssemblyName System.Drawing
    $bounds = [Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = [Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $memory = [IO.MemoryStream]::new()
    try {
        $graphics.CopyFromScreen($bounds.Location, [Drawing.Point]::Empty, $bounds.Size)
        $bitmap.Save($memory, [Drawing.Imaging.ImageFormat]::Png)
        $result = @{width=$bounds.Width; height=$bounds.Height; png=[Convert]::ToBase64String($memory.ToArray()); target=$target}
        [Console]::Write(($result | ConvertTo-Json -Compress -Depth 4))
    } finally { $memory.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
