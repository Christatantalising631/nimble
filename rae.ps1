param(
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $repoRoot

if (-not $SkipBuild) {
    Write-Host "Building release binary..."
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$runner = Join-Path $repoRoot "target\release\nimble.exe"
if (-not (Test-Path $runner)) {
    Write-Error "Missing release binary at $runner"
    exit 1
}

$failures = @()
$examples = Get-ChildItem -Path examples -Recurse -Filter *.nmb | Sort-Object FullName

foreach ($example in $examples) {
    Write-Host ""
    Write-Host "===== Running: $($example.Name) ====="
    Write-Host ""

    $output = & $runner run $example.FullName 2>&1
    if ($output) {
        $output | ForEach-Object { Write-Host $_ }
    }

    if ($LASTEXITCODE -ne 0 -or (($output | Out-String) -match "\[ERROR\]")) {
        $failures += $example.FullName
    }
}

if ($failures.Count -gt 0) {
    Write-Host ""
    Write-Host "Failures:"
    $failures | ForEach-Object { Write-Host $_ }
    exit 1
}
