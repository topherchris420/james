$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class JamesMouse {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}
'@
    [void][JamesMouse]::SetProcessDPIAware()
    Add-Type -AssemblyName System.Windows.Forms
    $bounds = [Windows.Forms.Screen]::PrimaryScreen.Bounds
    if ($bounds.Width -ne $request.width -or $bounds.Height -ne $request.height) { throw 'Screen size changed; capture and review the action again.' }
    $action = $request.action
    switch ($action.kind) {
        'click' {
            if ($action.x -lt 0 -or $action.y -lt 0 -or $action.x -ge $bounds.Width -or $action.y -ge $bounds.Height) { throw 'Click is outside the primary screen.' }
            if (-not [JamesMouse]::SetCursorPos([int]$action.x + $bounds.X, [int]$action.y + $bounds.Y)) { throw 'Could not position mouse.' }
            [JamesMouse]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
            [JamesMouse]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
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
