# Bateria IA F1-F10 (Fase 3 v2): turnos sinteticos via `wa_b` en paralelo.
# Uso: .\scripts\bateria-ia.ps1 [-Solo F1,F4] [-Base 18149575561]
# Cada escenario usa UN remitente virgen (un hilo por numero: evita el
# re-claveo H1 y aisla estados). Los jobs corren en paralelo y cada turno
# espera con polling al outbox (motivo='ia') en vez de sleeps fijos: la v1
# secuencial con esperas 120-150 s tardo ~70 min.
# Requiere backend en http://127.0.0.1:3000 (ver C:\tmp\mn-arrancar-backend.ps1).
param(
    [string[]]$Solo = @(),
    [string]$Base = "18149575561",
    [string]$Backend = "http://127.0.0.1:3000",
    [int]$TopeTurnoSeg = 150
)

$ErrorActionPreference = "Stop"
$Destino = "18149575416"

# Catalogo F1-F10: remitente = Base+indice (virgen por escenario).
$Escenarios = @(
    @{ Id = "F1";  Turnos = @("Hola, busco apartamento de 2 habitaciones en venta, mi presupuesto es 90000. Me llamo Luis Perez, mi numero es 34611111111") },
    @{ Id = "F2";  Turnos = @("Busco casa en el norte con 3 habitaciones") },
    @{ Id = "F3";  Turnos = @("Cuantos puestos tiene el local de Riberas del Caroni?") },
    @{ Id = "F4";  Turnos = @("Busco apartamento de 2 habitaciones, soy Ana Gomez 34622222222") },
    @{ Id = "F5";  Turnos = @("Cuentame un chiste") },
    @{ Id = "F6";  Turnos = @("Tienes algo en alquiler?") },
    @{ Id = "F7";  Turnos = @("Quiero hablar con una persona de verdad") },
    @{ Id = "F8";  Turnos = @("Donde queda la oficina? mi telefono es 34633333333") },
    @{ Id = "F9";  Turnos = @("Busco casa en venta", "mejor muestrame apartamentos en alquiler") },
    @{ Id = "F10"; Turnos = @("Buenas noches") }
)

$jobs = foreach ($e in $Escenarios) {
    if ($Solo.Count -gt 0 -and $Solo -notcontains $e.Id) { continue }
    $idx = [array]::IndexOf(($Escenarios | ForEach-Object { $_.Id }), $e.Id)
    # Sufijo desde la base (2 ultimos digitos + indice): `-Base 18149575581`
    # da 81, 82... sin colisionar con corridas anteriores.
    $sufijo = [int]"$Base".Substring(9) + $idx
    $remitente = "$Base".Substring(0, 9) + "$sufijo"
    Start-Job -Name "bateria-$($e.Id)" -ArgumentList $e, $remitente, $Destino, $Backend, $TopeTurnoSeg -ScriptBlock {
        param($esc, $rem, $dest, $be, $tope)
        $env:PGPASSWORD = "root"
        $psql = "C:\Program Files\PostgreSQL\18\bin\psql.exe"
        function Outbox-Ia([string]$sesion) {
            (& $psql -h localhost -U postgres -d glory_backend_inmobiliaria -tA -c `
                "SELECT count(*) FROM agent_outbox WHERE payload->>'session_id'='$sesion' AND payload->>'motivo'='ia';").Trim()
        }
        foreach ($t in $esc.Turnos) {
            $body = @{ numero_destino = $dest; remitente = $rem; texto = $t; via = "wa_b" } | ConvertTo-Json -Compress
            Invoke-RestMethod -Uri "$be/api/agent/whatsapp/webhook" -Method Post `
                -ContentType "application/json" -Body $body -TimeoutSec 30 | Out-Null
            # sesion del remitente (virgen: una sola fila por numero).
            $sesion = (& $psql -h localhost -U postgres -d glory_backend_inmobiliaria -tA -c `
                "SELECT session_id FROM canal_sesiones WHERE telefono='$rem';").Trim()
            $antes = Outbox-Ia $sesion
            # polling hasta ver texto IA nuevo o agotar tope (sin sleep fijo).
            $fin = (Get-Date).AddSeconds($tope)
            do {
                Start-Sleep -Seconds 5
                $ahora = Outbox-Ia $sesion
                if ([int]$ahora -gt [int]$antes) { break }
            } while ((Get-Date) -lt $fin)
        }
        return @{ Id = $esc.Id; Remitente = $rem }
    }
}

$jobs | Wait-Job | Out-Null
$resultados = $jobs | Receive-Job
$jobs | Remove-Job
return $resultados
