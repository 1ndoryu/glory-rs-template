<#
[028A-6 Fase 5] LEGACY — instalador del guard del repo, reemplazado por
    scripts/quality/install-global-runtime.mjs (sentinel install: runtime
    versionado + shims + perfiles + PATH de usuario). Se conserva SOLO para
    ramas antiguas y se retirará tras dos releases con rollback probado (plan
    028A-6 Fase 5). No usar en instalaciones nuevas.
#>
param(
    [switch]$Uninstall,
    [switch]$InstallProfile
)

$ErrorActionPreference = 'Stop'
$guardScript = Join-Path $PSScriptRoot 'global-cargo-guard.ps1'
$bashGuardScript = Join-Path $PSScriptRoot 'global-quality-guard.sh'
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
$bashProfilePaths = @(
    (Join-Path $HOME '.bashrc'),
    (Join-Path $HOME '.bash_profile')
) | Select-Object -Unique

$markerStart = '# >>> glory-quality-global-guard >>>'
$markerEnd = '# <<< glory-quality-global-guard <<<'
$bashMarkerStart = '# >>> glory-quality-global-bash-guard >>>'
$bashMarkerEnd = '# <<< glory-quality-global-bash-guard <<<'

function Convert-WindowsPathToBash {
    param([Parameter(Mandatory = $true)][string]$Path)
    $normalized = $Path.Replace('\', '/')
    if ($normalized -match '^([A-Za-z]):/(.*)$') {
        return "/$($matches[1].ToLowerInvariant())/$($matches[2])"
    }
    return $normalized
}

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

function Resolve-RealCommandPath {
    param([Parameter(Mandatory = $true)][string]$Name)
    $shimPath = Join-Path $shimDirectory "$Name.cmd"
    $command = Get-Command "$Name.cmd" -CommandType Application -ErrorAction Stop |
        Where-Object { $_.Source -ne $shimPath } |
        Select-Object -First 1
    if (-not $command) { throw "No se encontró el ejecutable real de $Name" }
    return $command.Source
}
if (-not $Uninstall) {
    $profileBlock = @"
$markerStart
. '$guardScript'
$markerEnd
"@
    $bashGuardPath = Convert-WindowsPathToBash -Path $bashGuardScript
    $bashProfileBlock = @"
$bashMarkerStart
if [ -f '$bashGuardPath' ]; then . '$bashGuardPath'; fi
$bashMarkerEnd
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
        foreach ($bashProfilePath in $bashProfilePaths) {
            $bashDirectory = Split-Path -Parent $bashProfilePath
            if (-not (Test-Path $bashDirectory)) { New-Item -ItemType Directory -Path $bashDirectory -Force | Out-Null }
            if (-not (Test-Path $bashProfilePath)) { New-Item -ItemType File -Path $bashProfilePath -Force | Out-Null }
            $bashContent = Normalize-ProfileText (Convert-MojibakeToUtf8 (Get-Content $bashProfilePath -Raw))
            $bashPattern = "(?s)" + [regex]::Escape($bashMarkerStart) + ".*?" + [regex]::Escape($bashMarkerEnd) + "\r?\n?"
            $bashContent = [regex]::Replace($bashContent, $bashPattern, '')
            $bashContent = ($bashContent.TrimEnd() + "`n" + $bashProfileBlock.Trim() + "`n")
            Set-Content -Path $bashProfilePath -Value $bashContent -Encoding utf8NoBOM
        }
        Write-Host "[glory-quality] Interceptor instalado en PowerShell ($($profilePaths -join ', ')) y Bash ($($bashProfilePaths -join ', '))" -ForegroundColor Green
    } else {
        Write-Host '[glory-quality] Shim PATH instalado; perfiles no modificados. Usa -InstallProfile solo tras revisar tu perfil.' -ForegroundColor Yellow
    }

    $realCargo = (Get-Command cargo.exe -CommandType Application | Select-Object -First 1).Source
    $realNpm = Resolve-RealCommandPath -Name 'npm'
    $realNpx = Resolve-RealCommandPath -Name 'npx'
    # [SNT-10/028A-16] node.cmd es ahora un shim del guard (entrypoints de
    # herramientas validadas); los shims npm/npx/cargo y el guard de bash lo
    # usan para evitar recursión al resolver el node real.
    $realNode = (Get-Command node.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    [Environment]::SetEnvironmentVariable('GLORY_REAL_CARGO', $realCargo, 'User')
    [Environment]::SetEnvironmentVariable('GLORY_REAL_NPM', $realNpm, 'User')
    [Environment]::SetEnvironmentVariable('GLORY_REAL_NPX', $realNpx, 'User')
    [Environment]::SetEnvironmentVariable('GLORY_REAL_NODE', $realNode, 'User')
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $pathEntries = @($userPath -split ';' | Where-Object { $_ })
    if ($pathEntries -notcontains $shimDirectory) {
        [Environment]::SetEnvironmentVariable('Path', (($shimDirectory + ';') + ($pathEntries -join ';')), 'User')
    }
    $env:GLORY_REAL_CARGO = $realCargo
    $env:GLORY_REAL_NPM = $realNpm
    $env:GLORY_REAL_NPX = $realNpx
    $env:GLORY_REAL_NODE = $realNode
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
        foreach ($bashProfilePath in $bashProfilePaths) {
            if (Test-Path $bashProfilePath) {
                $bashContent = Normalize-ProfileText (Convert-MojibakeToUtf8 (Get-Content $bashProfilePath -Raw))
                $bashPattern = "(?s)" + [regex]::Escape($bashMarkerStart) + ".*?" + [regex]::Escape($bashMarkerEnd) + "\r?\n?"
                $bashContent = [regex]::Replace($bashContent, $bashPattern, '')
                Set-Content -Path $bashProfilePath -Value $bashContent -Encoding utf8NoBOM
            }
        }
        Write-Host "[glory-quality] Interceptor retirado de los perfiles Bash" -ForegroundColor Yellow
    }
    Write-Host '[glory-quality] Se retiró el shim PATH si era administrado por este instalador.' -ForegroundColor Yellow
}
