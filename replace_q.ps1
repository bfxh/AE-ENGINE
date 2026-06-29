$f = 'd:\rj\wasteland_project\nova_render\src\backend\wgpu_backend.rs'
$c = [System.IO.File]::ReadAllText($f)
$c = $c -replace '~Q~', [char]34
[System.IO.File]::WriteAllText($f, $c, [System.Text.UTF8Encoding]::new($false))
Write-Output 'done'
