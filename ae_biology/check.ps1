Get-ChildItem 'd:\rj\wasteland_project\wasteland_biology\src\*.rs' | ForEach-Object {
    $tests = (Select-String -Path $_.FullName -Pattern 'fn test_' -AllMatches).Count
    $hasMod = (Select-String -Path $_.FullName -Pattern 'mod tests' -Quiet)
    $name = $_.Name
    $bytes = $_.Length
    Write-Output ("{0,-30} {1,8} bytes  tests={2,3}  mod={3}" -f $name, $bytes, $tests, $hasMod)
}
