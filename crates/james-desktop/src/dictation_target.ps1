$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class JamesDictationInspect {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct GUITHREADINFO {
        public uint cbSize, flags; public IntPtr active, focus, capture, menuOwner, moveSize, caret; public RECT caretRect;
    }
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool GetGUIThreadInfo(uint tid, ref GUITHREADINFO info);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
}
'@
    $front = [JamesDictationInspect]::GetForegroundWindow()
    $processId = [uint32]0
    $threadId = [JamesDictationInspect]::GetWindowThreadProcessId($front, [ref]$processId)
    $info = New-Object JamesDictationInspect+GUITHREADINFO
    $info.cbSize = [uint32][Runtime.InteropServices.Marshal]::SizeOf($info)
    if ($front -eq [IntPtr]::Zero -or $threadId -eq 0 -or -not [JamesDictationInspect]::GetGUIThreadInfo($threadId, [ref]$info) -or $info.focus -eq [IntPtr]::Zero) {
        throw 'The destination focus could not be identified. Choose a text field and try again.'
    }
    $focusProcessId = [uint32]0
    [void][JamesDictationInspect]::GetWindowThreadProcessId($info.focus, [ref]$focusProcessId)
    $title = [Text.StringBuilder]::new(257)
    $class = [Text.StringBuilder]::new(257)
    [void][JamesDictationInspect]::GetWindowText($front, $title, 257)
    [void][JamesDictationInspect]::GetClassName($info.focus, $class, 257)
    $name = ''
    $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
    if ($null -ne $process) { $name = [string]$process.ProcessName }
    $result = @{window=$front.ToInt64(); focused_window=$info.focus.ToInt64(); process_id=$processId; focused_process_id=$focusProcessId; process_name=$name; title=$title.ToString(); focus_class=$class.ToString()}
    [Console]::Write(($result | ConvertTo-Json -Compress))
} catch { [Console]::Error.Write($_.Exception.Message); exit 1 }
