Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$screen = [System.Windows.Forms.Screen]::PrimaryScreen
$bmp = New-Object System.Drawing.Bitmap($screen.WorkingArea.Width, $screen.WorkingArea.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($screen.WorkingArea.Location, [System.Drawing.Point]::Empty, $screen.WorkingArea.Size)
$bmp.Save("d:\rj\wasteland_project\godots_screenshot.png")
$g.Dispose()
$bmp.Dispose()
Write-Output "Screenshot saved to d:\rj\wasteland_project\godots_screenshot.png"