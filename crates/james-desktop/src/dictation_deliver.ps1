$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class JamesDictationDeliver {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct GUITHREADINFO {
        public uint cbSize, flags; public IntPtr active, focus, capture, menuOwner, moveSize, caret; public RECT caretRect;
    }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx, dy; public uint data, flags, time; public UIntPtr extra; }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort vk, scan; public uint flags, time; public UIntPtr extra; }
    [StructLayout(LayoutKind.Sequential)] public struct HARDWAREINPUT { public uint message; public ushort low, high; }
    [StructLayout(LayoutKind.Explicit)] public struct INPUTUNION {
        [FieldOffset(0)] public MOUSEINPUT mouse; [FieldOffset(0)] public KEYBDINPUT keyboard; [FieldOffset(0)] public HARDWAREINPUT hardware;
    }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public INPUTUNION data; }
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool GetGUIThreadInfo(uint tid, ref GUITHREADINFO info);
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vk);
    [DllImport("user32.dll", SetLastError=true)] public static extern uint SendInput(uint count, INPUT[] input, int size);
    public static INPUT[] Unicode(string text) {
        INPUT[] events = new INPUT[text.Length * 2];
        for (int i = 0; i < text.Length; i++) {
            events[i*2].type = 1;
            events[i*2].data.keyboard.scan = text[i];
            events[i*2].data.keyboard.flags = 4; // KEYEVENTF_UNICODE
            events[i*2+1] = events[i*2];
            events[i*2+1].data.keyboard.flags = 6; // UNICODE | KEYUP
        }
        return events;
    }
}
'@
    $text = [string]$request.text
    if ($text.Length -eq 0 -or $text.Length -gt 2000) { throw 'Reviewed text length is invalid. Nothing was sent.' }
    foreach ($c in $text.ToCharArray()) { if ([char]::IsControl($c)) { throw 'Reviewed text contains a control character. Nothing was sent.' } }
    $events = [JamesDictationDeliver]::Unicode($text)
    # Refuse held modifiers, which could change how the destination interprets input.
    foreach ($key in @(0x10,0x11,0x12,0x5B,0x5C)) {
        if (([int][JamesDictationDeliver]::GetAsyncKeyState($key) -band 0x8000) -ne 0) { throw 'Release modifier keys before sending. Nothing was sent.' }
    }
    $front = [JamesDictationDeliver]::GetForegroundWindow()
    $processId = [uint32]0
    $threadId = [JamesDictationDeliver]::GetWindowThreadProcessId($front, [ref]$processId)
    $info = New-Object JamesDictationDeliver+GUITHREADINFO
    $info.cbSize = [uint32][Runtime.InteropServices.Marshal]::SizeOf($info)
    if ($front -eq [IntPtr]::Zero -or $threadId -eq 0 -or $processId -eq [uint32]$request.own_process -or
        $front.ToInt64() -ne [int64]$request.target.window -or $processId -ne [uint32]$request.target.process_id -or
        -not [JamesDictationDeliver]::GetGUIThreadInfo($threadId, [ref]$info) -or $info.focus -eq [IntPtr]::Zero -or
        $info.focus.ToInt64() -ne [int64]$request.target.focused_window) {
        throw 'The destination window or focused control changed. Nothing was sent; review a new destination.'
    }
    $focusProcessId = [uint32]0
    [void][JamesDictationDeliver]::GetWindowThreadProcessId($info.focus, [ref]$focusProcessId)
    if ($focusProcessId -eq 0 -or $focusProcessId -eq [uint32]$request.own_process -or $focusProcessId -ne [uint32]$request.target.focused_process_id) {
        throw 'The destination control changed. Nothing was sent; review a new destination.'
    }
    # Native focused HWND cannot identify a field inside a custom control, and
    # focus could change after this check. The preview must disclose both limits.
    $sent = [JamesDictationDeliver]::SendInput([uint32]$events.Length, $events, [Runtime.InteropServices.Marshal]::SizeOf([type][JamesDictationDeliver+INPUT]))
    if ($sent -ne $events.Length) { throw 'Windows accepted only part or none of the input. Inspect the destination before retrying; some text may have been inserted.' }
} catch { [Console]::Error.Write($_.Exception.Message); exit 1 }
