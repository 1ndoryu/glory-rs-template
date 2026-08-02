param(
    [switch]$Uninstall,
    [switch]$InstallProfile
)

$ErrorActionPreference = 'Stop'
$guardScript = Join-Path $PSScriptRoot 'global-cargo-guard.ps1'
$shimDirectory = $PSScriptRoot
$currentProfile = [string]$PROFILE
if ([string]::IsNullOrWhiteSpace($currentProfile)) {
    $currentProfile = Join-Path $HOME 'Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1'
}
$profileParent = Split-Path -Parent (Split-Path -Parent $currentProfile)
$profilePaths = @(
    $currentProfile,
    (Join-Path $profileParent 'PowerShell\Microsoft.PowerShell_profile.ps1'),
    (Join-Path $profileParent 'WindowsPowerShell\Microsoft.PowerShell_profile.ps1')
) | Where-Object { $_ } | Select-Object -Unique

$markerStart = '# >>> glory-quality-global-guard >>>'
$markerEnd = '# <<< glory-quality-global-guard <<<'

function Convert-MojibakeToUtf8 {
    param([string]$Text)
    if ($Text -notmatch 'Ã|Â|â') { return $Text }
    $legacy = [System.Text.Encoding]::GetEncoding(1252)
    return [System.Text.Encoding]::UTF8.GetString($legacy.GetBytes($Text))
}

function Normalize-ProfileText {
    param([string]$Text)
    # Algunos perfiles antiguos guardaron `` `n`` como texto literal, lo que
    # convierte la siguiente asignación en un comando inválido al iniciar.
    $literalNewLine = [string][char]96 + 'n'
    return $Text.Replace($literalNewLine, [Environment]::NewLine)
}
if (-not $Uninstall) {
    $profileBlock = @"
$markerStart
. '$guardScript'
$markerEnd
"@
    $pattern = "(?s)" + [regex]::Escape($markerStart) + ".*?" + [regex]::Escape($markerEnd) + "\r?\n?"
    if ($InstallProfile) {
        foreach ($profilePath in $profilePaths) {
            $profileDirectory = Split-Path -Parent $profilePath
            if (-not (Test-Path $profileDirectory)) { New-Item -ItemType Directory -Path $profileDirectory -Force | Out-Null }
            if (-not (Test-Path $profilePath)) { New-Item -ItemType File -Path $profilePath -Force | Out-Null }
            $profileContent = Normalize-ProfileText (Convert-MojibakeToUtf8 (Get-Content $profilePath -Raw))
            $profileContent = [regex]::Replace($profileContent, $pattern, '')
            $profileContent = ($profileContent.TrimEnd() + "`r`n" + $profileBlock.Trim() + "`r`n")
            Set-Content -Path $profilePath -Value $profileContent -Encoding utf8NoBOM
        }
        Write-Host "[glory-quality] Interceptor de perfil instalado en $($profilePaths -join ', ')" -ForegroundColor Green
    } else {
        Write-Host '[glory-quality] Shim PATH instalado; perfiles no modificados. Usa -InstallProfile solo tras revisar tu perfil.' -ForegroundColor Yellow
    }

    $realCargo = (Get-Command cargo.exe -CommandType Application | Select-Object -First 1).Source
    [Environment]::SetEnvironmentVariable('GLORY_REAL_CARGO', $realCargo, 'User')
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $pathEntries = @($userPath -split ';' | Where-Object { $_ })
    if ($pathEntries -notcontains $shimDirectory) {
        [Environment]::SetEnvironmentVariable('Path', (($shimDirectory + ';') + ($pathEntries -join ';')), 'User')
    }
    $env:GLORY_REAL_CARGO = $realCargo
    if (($env:Path -split ';') -notcontains $shimDirectory) { $env:Path = "$shimDirectory;$env:Path" }
    Write-Host '[glory-quality] Cooldown global: 3 horas por proyecto; usa --allow-heavy solo manualmente.' -ForegroundColor Yellow
} else {
    $pattern = "(?s)" + [regex]::Escape($markerStart) + ".*?" + [regex]::Escape($markerEnd) + "\r?\n?"
    if ($InstallProfile) {
        foreach ($profilePath in $profilePaths) {
            if (Test-Path $profilePath) {
                $profileContent = Normalize-ProfileText (Convert-MojibakeToUtf8 (Get-Content $profilePath -Raw))
                $profileContent = [regex]::Replace($profileContent, $pattern, '')
                Set-Content -Path $profilePath -Value $profileContent -Encoding utf8NoBOM
            }
        }
        Write-Host "[glory-quality] Interceptor retirado de los perfiles PowerShell" -ForegroundColor Yellow
    }
    Write-Host '[glory-quality] Se retiró el shim PATH si era administrado por este instalador.' -ForegroundColor Yellow
}
