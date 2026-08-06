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
if not defined GLORY_REAL_NPM (
  for /f "delims=" %%I in ('where npm.cmd 2^>nul') do if /I not "%%~fI"=="%~f0" if not defined GLORY_REAL_NPM set "GLORY_REAL_NPM=%%~fI"
)
if not defined GLORY_REAL_NPM (
  echo [glory-quality] No se encontro el npm real fuera del shim. 1>&2
  exit /b 127
)
if not defined GLORY_REAL_NODE (
  for /f "delims=" %%I in ('where node.exe 2^>nul') do if not "%%~fI"=="%~dp0node.cmd" if not defined GLORY_REAL_NODE set "GLORY_REAL_NODE=%%~fI"
)
if not defined GLORY_REAL_NODE (
  echo [glory-quality] No se encontro el node real fuera del shim. 1>&2
  exit /b 127
)
"%GLORY_REAL_NODE%" "%GLORY_GUARD_ROOT%quality-command-guard.mjs" --project-root "%CD%" --executable npm -- %*
if errorlevel 1 exit /b %ERRORLEVEL%
"%GLORY_REAL_NPM%" %*
exit /b %ERRORLEVEL%
