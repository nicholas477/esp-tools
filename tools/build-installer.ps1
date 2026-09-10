$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$guiProject = Join-Path $repositoryRoot 'esp-tools-gui\esp-tools-gui\esp-tools-gui.csproj'
$guiOutputDirectory = Join-Path $repositoryRoot 'esp-tools-gui\esp-tools-gui\bin\x64\Release\net48'
$stagingDirectory = Join-Path $repositoryRoot 'target\installer'

Push-Location $repositoryRoot
try {
    & cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build exited with code $LASTEXITCODE."
    }

    & dotnet build $guiProject --configuration Release --property:Platform=x64
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet build exited with code $LASTEXITCODE."
    }

    New-Item -ItemType Directory -Force -Path $stagingDirectory | Out-Null
    Get-ChildItem -Path $stagingDirectory -File | Remove-Item

    Copy-Item 'target\release\esp-tools.exe' $stagingDirectory
    Copy-Item 'target\release\esp_tools_lib.dll' $stagingDirectory
    Copy-Item (Join-Path $guiOutputDirectory 'esp-tools-gui.exe') $stagingDirectory
    Copy-Item (Join-Path $guiOutputDirectory 'esp-tools-gui.exe.config') $stagingDirectory
    Copy-Item (Join-Path $guiOutputDirectory 'Newtonsoft.Json.dll') $stagingDirectory

    & cargo wix --target-bin-dir $stagingDirectory --no-build --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "cargo wix exited with code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}