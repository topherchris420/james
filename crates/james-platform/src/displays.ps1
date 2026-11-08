$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
try {
    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -Namespace JamesDisplays -Name Native -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
    # Match the capture helper's DPI awareness so sizes equal the pixels captured.
    [void][JamesDisplays.Native]::SetProcessDPIAware()
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
        $items += (ConvertTo-Json -InputObject $row -Compress)
    }
    [Console]::Out.Write("[" + ($items -join ",") + "]")
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
