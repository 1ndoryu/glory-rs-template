Option Explicit

If WScript.Arguments.Count <> 3 Then
    WScript.Quit 64
End If

Dim shell, cleanScript, targetDir, maxTotalMb, quote, command, exitCode
Set shell = CreateObject("WScript.Shell")
cleanScript = WScript.Arguments(0)
targetDir = WScript.Arguments(1)
maxTotalMb = WScript.Arguments(2)
quote = Chr(34)

' [257A-8] WScript no reserva una consola. El segundo argumento de Run en 0
' mantiene oculto también a PowerShell desde su creación, evitando el destello
' que -WindowStyle Hidden no puede impedir durante el arranque del proceso.
command = "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File " _
    & quote & cleanScript & quote _
    & " -TargetDirs " & quote & targetDir & quote _
    & " -MaxTotalMB " & maxTotalMb

exitCode = shell.Run(command, 0, True)
WScript.Quit exitCode
