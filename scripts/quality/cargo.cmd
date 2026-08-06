@echo off
rem ============================================================
rem [028A-6 Fase 5] LEGACY — adaptador de migración del guard.
rem El runtime global de Sentinel (sentinel install, %LOCALAPPDATA%\GlorySentinel)
rem ya genera sus propios shims; este shim del repo se conserva SOLO para
rem ramas antiguas y se retirará tras dos releases con rollback probado
rem (plan 028A-6 Fase 5). No modificar su lógica: la fuente canónica es el
rem runtime. Reemplazo canónico: sentinel guard / sentinel check.
rem ============================================================
setlocal
set "GLORY_GUARD_ROOT=%~dp0"
set "GLORY_REAL_CARGO=%GLORY_REAL_CARGO%"
if not defined GLORY_REAL_CARGO set "GLORY_REAL_CARGO=C:\Users\Owner\.cargo\bin\cargo.exe"
if not defined GLORY_REAL_NODE (
  for /f "delims=" %%I in ('where node.exe 2^>nul') do if not "%%~fI"=="%~dp0node.cmd" if not defined GLORY_REAL_NODE set "GLORY_REAL_NODE=%%~fI"
)
if not defined GLORY_REAL_NODE (
  echo [glory-quality] No se encontro el node real fuera del shim. 1>&2
  exit /b 127
)
"%GLORY_REAL_NODE%" "%GLORY_GUARD_ROOT%quality-command-guard.mjs" --project-root "%CD%" --executable cargo -- %*
if errorlevel 1 exit /b %ERRORLEVEL%
"%GLORY_REAL_NODE%" "%GLORY_GUARD_ROOT%heavy-run-guard.mjs" --execute-cargo --project-root "%CD%" --cargo-path "%GLORY_REAL_CARGO%" -- %*
exit /b %ERRORLEVEL%
