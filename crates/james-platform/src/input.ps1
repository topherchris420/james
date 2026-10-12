$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class JamesInput {
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr hWnd, uint flags);
}
'@
    [void][JamesInput]::SetProcessDPIAware()
    Add-Type -AssemblyName System.Windows.Forms
    $bounds = [Windows.Forms.Screen]::PrimaryScreen.Bounds
    if ($bounds.Width -ne $request.width -or $bounds.Height -ne $request.height) { throw 'Screen size changed; capture and review the action again.' }

    # Checked immediately before input. A window that jumps in front within
    # the next few milliseconds is not prevented; this catches the common
    # mistakes of not switching back, or switching to the wrong window.
    $front = [JamesInput]::GetForegroundWindow()
    $frontProcess = [uint32]0
    [void][JamesInput]::GetWindowThreadProcessId($front, [ref]$frontProcess)
    if ($null -ne $request.james_process_id -and $frontProcess -eq [uint32]$request.james_process_id) {
        throw 'James itself is in front, so nothing was sent. Switch to the app you captured during the countdown.'
    }
    $target = $request.target
    if ($null -ne $target) {
        if ($front.ToInt64() -ne [int64]$target.window -or $frontProcess -ne [uint32]$target.process_id) {
            throw 'The window in front is not the one in your screenshot, so nothing was sent. Capture the screen again with the right app in front.'
        }
    }

    $action = $request.action
    switch ($action.kind) {
        'click' {
            if ($action.x -lt 0 -or $action.y -lt 0 -or $action.x -ge $bounds.Width -or $action.y -ge $bounds.Height) { throw 'Click is outside the primary screen.' }
            $point = New-Object JamesInput+POINT
            $point.X = [int]$action.x + $bounds.X
            $point.Y = [int]$action.y + $bounds.Y
            if ($null -ne $target) {
                # GA_ROOT (2): the top-level window that owns the point.
                $under = [JamesInput]::GetAncestor([JamesInput]::WindowFromPoint($point), 2)
                if ($under.ToInt64() -ne [int64]$target.window) { throw 'The point you approved is not on the window in your screenshot, so nothing was sent.' }
            }
            if (-not [JamesInput]::SetCursorPos($point.X, $point.Y)) { throw 'Could not position mouse.' }
            [JamesInput]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
            [JamesInput]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
        }
        'key' {
            $keys = @{enter='{ENTER}';tab='{TAB}';escape='{ESC}';backspace='{BACKSPACE}';delete='{DELETE}';up='{UP}';down='{DOWN}';left='{LEFT}';right='{RIGHT}';home='{HOME}';end='{END}';space=' '}
            if (-not $keys.ContainsKey([string]$action.key)) { throw 'Unsupported key.' }
            [Windows.Forms.SendKeys]::SendWait($keys[[string]$action.key])
        }
        'type_text' {
            $text = [string]$action.text
            if ($text.Length -eq 0 -or $text.Length -gt 2000) { throw 'Invalid text length.' }
            $escaped = [Text.StringBuilder]::new()
            foreach ($char in $text.ToCharArray()) {
                if ([char]::IsControl($char)) { throw 'Control characters are not permitted in typed text.' }
                if ($char -eq '{') { [void]$escaped.Append('{{}') }
                elseif ($char -eq '}') { [void]$escaped.Append('{}}') }
                elseif ('+^%~()[]'.Contains([string]$char)) { [void]$escaped.Append('{' + $char + '}') }
                else { [void]$escaped.Append($char) }
            }
            [Windows.Forms.SendKeys]::SendWait($escaped.ToString())
        }
        default { throw 'Unsupported action.' }
    }
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
