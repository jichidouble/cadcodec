param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('gcc', 'cl')]
    [string]$Compiler,
    [Parameter(Mandatory = $true)]
    [string]$Dll,
    [Parameter(Mandatory = $true)]
    [string]$ImportLibrary
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$source = Join-Path $repoRoot 'tests/ffi/c_header.c'
$includeDir = Join-Path $repoRoot 'include'

foreach ($path in @($source, $includeDir, $Dll, $ImportLibrary)) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Required smoke-test path does not exist: $path"
    }
}

$compilerCommand = Get-Command $Compiler -ErrorAction SilentlyContinue
if ($null -eq $compilerCommand) {
    throw "Compiler '$Compiler' is not available on PATH."
}

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ('acadrust-ffi-header-smoke-' + [guid]::NewGuid())
$executable = Join-Path $tempDir 'c_header_smoke.exe'
$oldPath = $env:PATH
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

try {
    if ($Compiler -eq 'gcc') {
        $arguments = @($source, "-I$includeDir", $ImportLibrary, '-o', $executable)
    }
    else {
        $arguments = @('/nologo', "/I$includeDir", $source, $ImportLibrary, "/Fe:$executable")
    }

    & $compilerCommand.Source @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Compiler failed to compile or link the C header smoke test (exit $LASTEXITCODE)."
    }

    $dllDirectory = Split-Path -Parent (Resolve-Path -LiteralPath $Dll)
    $env:PATH = "$dllDirectory$([System.IO.Path]::PathSeparator)$oldPath"
    & $executable
    if ($LASTEXITCODE -ne 0) {
        throw "C header smoke test failed at runtime (exit $LASTEXITCODE)."
    }
}
finally {
    $env:PATH = $oldPath
    Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
