param([switch]$Check)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$cbindgen = Get-Command cbindgen -ErrorAction SilentlyContinue
if ($null -eq $cbindgen) {
    Write-Error 'cbindgen is required on PATH. Install it with: cargo install cbindgen --version 0.29.0 --locked'
    exit 1
}

Push-Location $repoRoot
$temporaryHeader = $null
try {
    if ($Check) {
        $temporaryHeader = Join-Path ([System.IO.Path]::GetTempPath()) ("acadrust-dwg-api-{0}.h" -f [guid]::NewGuid())
        & cbindgen --quiet --config cbindgen.toml src/ffi/mod.rs --output $temporaryHeader
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }

        $checkedInHeader = Join-Path $repoRoot 'include/dwg_api.h'
        if (-not (Test-Path -LiteralPath $checkedInHeader)) {
            Write-Error "Generated header is missing: $checkedInHeader"
            exit 1
        }

        $generatedBytes = [System.IO.File]::ReadAllBytes($temporaryHeader)
        $checkedInBytes = [System.IO.File]::ReadAllBytes($checkedInHeader)
        if (-not [System.Linq.Enumerable]::SequenceEqual($generatedBytes, $checkedInBytes)) {
            Write-Error 'include/dwg_api.h is out of date; run tools/generate_ffi_header.ps1'
            exit 1
        }

        Write-Output 'include/dwg_api.h is up to date.'
    }
    else {
        & cbindgen --quiet --config cbindgen.toml src/ffi/mod.rs --output include/dwg_api.h
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
}
finally {
    Pop-Location
    if ($null -ne $temporaryHeader) {
        Remove-Item -LiteralPath $temporaryHeader -Force -ErrorAction SilentlyContinue
    }
}
