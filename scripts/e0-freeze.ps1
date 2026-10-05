#Requires -Version 7
<#
E0.2 03AA-3 — Freeze cifrado directo del fork marketplace-*.
Sustituye `7z a -p -mhe=on` (7-Zip no instalado): ZIP en memoria +
AES-256-GCM (PBKDF2-SHA256 200k). El claro NUNCA toca disco
(ni C:\tmp ni Destino): solo existe en buffers de memoria.
Clave aleatoria generada aqui y guardada en Credential Manager de Windows
(cmdkey /generic:mp-privado-freeze); recuperacion posterior: Panel de
control → Cuentas de usuario → Administrador de credenciales → genéricas.
Uso: pwsh -NoProfile -File scripts/e0-freeze.ps1 -ForkDir <dir> -Destino <dir>
#>
param(
  [Parameter(Mandatory = $true)][string]$ForkDir,
  [Parameter(Mandatory = $true)][string]$Destino
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression

if (-not (Test-Path -LiteralPath $ForkDir -PathType Container)) { throw "ForkDir no existe: $ForkDir" }
if (-not (Test-Path -LiteralPath $Destino -PathType Container)) { throw "Destino no existe: $Destino" }

$ficheros = @(Get-ChildItem -Path (Join-Path $ForkDir 'marketplace-*.ts') -File | Sort-Object Name)
if ($ficheros.Count -eq 0) { throw "Sin marketplace-*.ts en $ForkDir" }

# ZIP solo en memoria
$ms = New-Object System.IO.MemoryStream
$zip = New-Object System.IO.Compression.ZipArchive($ms, [System.IO.Compression.ZipArchiveMode]::Create, $true)
$originales = @{}
try {
  foreach ($f in $ficheros) {
    $bytes = [System.IO.File]::ReadAllBytes($f.FullName)
    $originales[$f.Name] = (New-Object System.Security.Cryptography.SHA256Managed).ComputeHash($bytes)
    $entrada = $zip.CreateEntry($f.Name, [System.IO.Compression.CompressionLevel]::Optimal)
    $flujo = $entrada.Open()
    try { $flujo.Write($bytes, 0, $bytes.Length) } finally { $flujo.Close() }
    [Array]::Clear($bytes, 0, $bytes.Length)
  }
} finally { $zip.Dispose() }
$zipBytes = $ms.ToArray(); $ms.Dispose()

# Clave aleatoria → Credential Manager (keyring del SO)
$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$claveBytes = New-Object byte[] 24; $rng.GetBytes($claveBytes)
$clave = [Convert]::ToBase64String($claveBytes)
[Array]::Clear($claveBytes, 0, $claveBytes.Length)
& cmdkey /generic:mp-privado-freeze /user:mp /pass:$clave | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'cmdkey fallo al guardar la clave' }

# PBKDF2-SHA256 200k → AES-256-GCM
$sal = New-Object byte[] 16; $rng.GetBytes($sal)
$kdf = New-Object System.Security.Cryptography.Rfc2898DeriveBytes($clave, $sal, 200000, [System.Security.Cryptography.HashAlgorithmName]::SHA256)
$llave = $kdf.GetBytes(32); $kdf.Dispose()
$nonce = New-Object byte[] 12; $rng.GetBytes($nonce); $rng.Dispose()
$aes = [System.Security.Cryptography.AesGcm]::new($llave)
$cifrado = New-Object byte[] ($zipBytes.Length); $etiqueta = New-Object byte[] 16
$aes.Encrypt($nonce, $zipBytes, $cifrado, $etiqueta); $aes.Dispose()
[Array]::Clear($zipBytes, 0, $zipBytes.Length)
[Array]::Clear($llave, 0, $llave.Length)

# Contenedor MPF1: magia4 + sal16 + nonce12 + cifrado + tag16
$contenedor = New-Object System.IO.MemoryStream
$contenedor.Write([System.Text.Encoding]::ASCII.GetBytes('MPF1'), 0, 4)
$contenedor.Write($sal, 0, 16); $contenedor.Write($nonce, 0, 12)
$contenedor.Write($cifrado, 0, $cifrado.Length); $contenedor.Write($etiqueta, 0, 16)
$blob = $contenedor.ToArray(); $contenedor.Dispose()
[Array]::Clear($cifrado, 0, $cifrado.Length)

$fecha = Get-Date -Format 'yyyy-MM-dd_HHmm'
$ruta = Join-Path $Destino "mp-freeze-$fecha.mpfreeze"
[System.IO.File]::WriteAllBytes($ruta, $blob)

# Verificacion round-trip en memoria (descifra y compara SHA256 por entrada)
$kdf2 = New-Object System.Security.Cryptography.Rfc2898DeriveBytes($clave, $sal, 200000, [System.Security.Cryptography.HashAlgorithmName]::SHA256)
$llave2 = $kdf2.GetBytes(32); $kdf2.Dispose()
$aes2 = [System.Security.Cryptography.AesGcm]::new($llave2)
$claro = New-Object byte[] ($blob.Length - 32 - 16)
$aes2.Decrypt($blob[20..31], $blob[32..($blob.Length - 17)], $blob[($blob.Length - 16)..($blob.Length - 1)], $claro)
$aes2.Dispose(); [Array]::Clear($llave2, 0, $llave2.Length)
$ms2 = New-Object System.IO.MemoryStream(,$claro)
$zip2 = New-Object System.IO.Compression.ZipArchive($ms2, [System.IO.Compression.ZipArchiveMode]::Read)
try {
  foreach ($entrada in $zip2.Entries) {
    $lector = New-Object System.IO.BinaryReader($entrada.Open())
    try { $datos = $lector.ReadBytes([int]$entrada.Length) } finally { $lector.Close() }
    $h = (New-Object System.Security.Cryptography.SHA256Managed).ComputeHash($datos)
    $esperado = [Convert]::ToBase64String($originales[$entrada.Name])
    if ([Convert]::ToBase64String($h) -ne $esperado) { throw "Round-trip distinto en $($entrada.Name)" }
  }
} finally { $zip2.Dispose(); $ms2.Dispose() }
[Array]::Clear($claro, 0, $claro.Length)

# Bloqueo: solo el usuario actual (hereda nada). whoami con dominio:
# $env:USERNAME solo ("owner") lo rechaza icacls ("Parametro no valido").
& icacls $Destino /inheritance:r /grant:r "$(whoami):(F)" | Out-Null

$sha = (Get-FileHash -LiteralPath $ruta -Algorithm SHA256).Hash
@(
  "freeze=$ruta",
  "sha256=$sha",
  "fecha=$(Get-Date -Format 'yyyy-MM-dd HH:mm')",
  "ficheros=$($ficheros.Count)",
  "algoritmo=AES-256-GCM/PBKDF2-SHA256-200k (contenedor MPF1)",
  "clave=Credential Manager generica mp-privado-freeze",
  "roundtrip=OK"
) | Set-Content -LiteralPath (Join-Path $Destino 'freeze-info.txt') -Encoding utf8

"FREEZE_OK ficheros=$($ficheros.Count) roundtrip=OK"
"RUTA=$ruta"
"SHA256=$sha"
