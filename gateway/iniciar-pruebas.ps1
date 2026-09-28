# 289A-1: arranca backend + gateway Baileys en ventanas propias para
# probar con WhatsApp real (numero A + B temporal). Los procesos son tuyos:
# sobreviven al agente. Cierra las ventanas al terminar de probar.
$ErrorActionPreference = "Stop"
$raiz = Split-Path $PSScriptRoot -Parent
$exe = "C:\tmp\glory-target\glory_backend_inmobiliaria\debug\glory-backend.exe"
if (-not (Test-Path $exe)) { throw "Falta el backend: $exe (avisa al agente)" }

$secreto = -join ((48..57)+(65..90)+(97..122) | Get-Random -Count 24 | ForEach-Object { [char]$_ })
$env:WA_WEBHOOK_SECRETO = $secreto
$env:GATEWAY_SEND_SECRET = $secreto
$env:GLORY_ALERT_GATEWAY_URL = "http://127.0.0.1:3102/send"
$env:RUST_LOG = "info"
$env:GATEWAY_PORT = "3102"
$env:BACKEND_WEBHOOK_URL = "http://127.0.0.1:3000/api/agent/whatsapp/webhook"
$env:SESSION_A_NUMBER = "584120825234"
$env:SESSION_B_NUMBER = "18149575416"

Start-Process -FilePath $exe -WorkingDirectory $raiz
Start-Process -FilePath "node" -ArgumentList "src/index.mjs" -WorkingDirectory $PSScriptRoot

for ($i = 0; $i -lt 15; $i++) {
    Start-Sleep 2
    try {
        $h = Invoke-WebRequest -Uri "http://127.0.0.1:3000/api/health" -TimeoutSec 3 -UseBasicParsing | ConvertFrom-Json
        if ($h.status -eq "ok") { break }
    } catch { }
}
Write-Host ""
Write-Host "Backend: http://127.0.0.1:3000  Gateway: http://127.0.0.1:3102"
Write-Host "1) En la ventana del gateway saldran 2 QR (o abre estos PNG):"
Write-Host "   $PSScriptRoot\qr-wa_a.png  <- escanear con el telefono A (0412 0825234)"
Write-Host "   $PSScriptRoot\qr-wa_b.png  <- escanear con el +1 (814) 957-5416 (B temporal)"
Write-Host "   (WhatsApp > Dispositivos vinculados > Vincular; si un QR vence, se regenera solo)"
Write-Host "2) Escribe al A desde otro telefono y al +1 para probar el modo inicial."
Write-Host "3) Avisame 'vinculado' cuando ambas sesiones digan 'sesion abierta'."
