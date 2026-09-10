$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$readmePath = Join-Path $repositoryRoot 'README.md'
$helpText = (& cmd.exe /c 'cargo run --release -- --help 2>nul') -join "`n"

if ($LASTEXITCODE -ne 0) {
    throw "Unable to generate CLI help; cargo run exited with code $LASTEXITCODE."
}

$replacement = "<!-- BEGIN GENERATED HELP -->`n``````text`n$helpText`n```````n<!-- END GENERATED HELP -->"
$readme = [System.IO.File]::ReadAllText($readmePath)
$updatedReadme = [System.Text.RegularExpressions.Regex]::Replace(
    $readme,
    '(?s)<!-- BEGIN GENERATED HELP -->.*?<!-- END GENERATED HELP -->',
    $replacement
)

if ($updatedReadme -eq $readme) {
    return
}

$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($readmePath, $updatedReadme, $utf8WithoutBom)