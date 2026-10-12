$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class JamesCapture {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    // How many of one process's top-level windows are shown, not minimized,
    // and overlap the given screen area.
    public static int Covering(uint processId, int left, int top, int right, int bottom) {
        int count = 0;
        EnumWindows(delegate (IntPtr hWnd, IntPtr lParam) {
            uint owner;
            RECT r;
            GetWindowThreadProcessId(hWnd, out owner);
            if (owner == processId && IsWindowVisible(hWnd) && !IsIconic(hWnd) && GetWindowRect(hWnd, out r)
                && r.Left < right && r.Right > left && r.Top < bottom && r.Bottom > top) {
                count++;
            }
            return true;
        }, IntPtr.Zero);
        return count;
    }
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
    # James minimized itself before this helper started, but the window
    # animates away and the compositor drops it only from a later frame. Wait
    # until none of James's windows has covered the screen for a few checks in
    # a row (bounded), then for composition, so James stays out of the picture.
    $jamesId = [uint32]$request.james_process_id
    $covering = { [JamesCapture]::Covering($jamesId, $bounds.Left, $bounds.Top, $bounds.Right, $bounds.Bottom) }
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $clear = 0
    while ($true) {
        if ((& $covering) -eq 0) { $clear++ } else { $clear = 0 }
        if ($clear -ge $request.stable_polls -or $clock.ElapsedMilliseconds -ge $request.ceiling_ms) { break }
        Start-Sleep -Milliseconds $request.poll_ms
    }
    foreach ($wait in $request.composition_ms) { Start-Sleep -Milliseconds $wait }
    $jamesHidden = (& $covering) -eq 0
    $bitmap = [Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $memory = [IO.MemoryStream]::new()
    try {
        $graphics.CopyFromScreen($bounds.Location, [Drawing.Point]::Empty, $bounds.Size)
        $bitmap.Save($memory, [Drawing.Imaging.ImageFormat]::Png)
        $result = @{width=$bounds.Width; height=$bounds.Height; png=[Convert]::ToBase64String($memory.ToArray()); target=$target; james_hidden=$jamesHidden}
        [Console]::Write(($result | ConvertTo-Json -Compress -Depth 4))
    } finally { $memory.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
