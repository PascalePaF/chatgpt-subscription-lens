[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$VersionFile = (Get-Content -LiteralPath (Join-Path $RepoRoot 'VERSION') -Raw).Trim()
if ($VersionFile -ne $Version) {
    throw "VERSION contains '$VersionFile', expected '$Version'."
}

$PublishPath = Join-Path $RepoRoot 'artifacts\publish'
$ReleasePath = Join-Path $RepoRoot 'release'
New-Item -ItemType Directory -Force -Path $PublishPath, $ReleasePath | Out-Null

dotnet build (Join-Path $RepoRoot 'SubscriptionLens.sln') -c Release -p:Version=$Version
if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }

dotnet run --project (Join-Path $RepoRoot 'tests\SubscriptionLens.Tests\SubscriptionLens.Tests.csproj') -c Release --no-build
if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }

dotnet publish (Join-Path $RepoRoot 'src\SubscriptionLens.App\SubscriptionLens.App.csproj') `
    -c Release `
    -r win-x64 `
    --self-contained true `
    -p:Version=$Version `
    -p:AssemblyVersion="$Version.0" `
    -p:FileVersion="$Version.0" `
    -o $PublishPath
if ($LASTEXITCODE -ne 0) { throw 'Publish failed.' }

$Nsis = 'C:\Program Files (x86)\NSIS\makensis.exe'
if (-not (Test-Path -LiteralPath $Nsis)) {
    $NsisCommand = Get-Command makensis -ErrorAction SilentlyContinue
    if ($null -eq $NsisCommand) { throw 'NSIS makensis was not found.' }
    $Nsis = $NsisCommand.Source
}

Push-Location (Join-Path $RepoRoot 'installer')
try {
    & $Nsis /INPUTCHARSET UTF8 "/DAPP_VERSION=$Version" 'SubscriptionLens.nsi'
    if ($LASTEXITCODE -ne 0) { throw 'Installer build failed.' }
}
finally {
    Pop-Location
}

$Installer = Join-Path $ReleasePath "SubscriptionLens-v$Version-windows-x64-setup.exe"
if (-not (Test-Path -LiteralPath $Installer)) { throw "Installer was not produced: $Installer" }
$Hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Installer).Hash.ToLowerInvariant()
$ChecksumPath = "$Installer.sha256"
Set-Content -LiteralPath $ChecksumPath -Value "$Hash  $(Split-Path -Leaf $Installer)" -Encoding ascii -NoNewline

[pscustomobject]@{
    Version = $Version
    Installer = $Installer
    Sha256 = $Hash
    Bytes = (Get-Item -LiteralPath $Installer).Length
}
