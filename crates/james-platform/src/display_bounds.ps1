$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    Add-Type -Namespace JamesDisplays -Name Native -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr context);'
    # Per-monitor awareness is needed for physical positions on mixed-DPI
    # desktops. If unavailable, leave bounds out instead of guessing.
    $physicalBounds = $false
    try { $physicalBounds = [JamesDisplays.Native]::SetProcessDpiAwarenessContext([IntPtr]::new(-4)) } catch { }
    Add-Type -AssemblyName System.Windows.Forms
    $screens = [Windows.Forms.Screen]::AllScreens
    # Build the JSON array by hand so a single monitor is still an array.
    $items = @()
    for ($i = 0; $i -lt $screens.Count; $i++) {
        $s = $screens[$i]
        $b = $s.Bounds
        $name = if ($s.Primary) { 'Primary display' } else { "Display $($i + 1)" }
        $row = [pscustomobject]@{
            id      = $s.DeviceName
            label   = "$name  $($b.Width) x $($b.Height)"
            primary = [bool]$s.Primary
        }
        if ($physicalBounds) {
            $row | Add-Member -NotePropertyName x -NotePropertyValue $b.X
            $row | Add-Member -NotePropertyName y -NotePropertyValue $b.Y
            $row | Add-Member -NotePropertyName width -NotePropertyValue $b.Width
            $row | Add-Member -NotePropertyName height -NotePropertyValue $b.Height
        }
        $items += (ConvertTo-Json -InputObject $row -Compress)
    }
    [Console]::Out.Write("[" + ($items -join ",") + "]")
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
