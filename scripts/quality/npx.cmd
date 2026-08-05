@echo off
setlocal
set "GLORY_GUARD_ROOT=%~dp0"
if not defined GLORY_REAL_NPX (
  for /f "delims=" %%I in ('where npx.cmd 2^>nul') do if /I not "%%~fI"=="%~f0" if not defined GLORY_REAL_NPX set "GLORY_REAL_NPX=%%~fI"
)
if not defined GLORY_REAL_NPX (
  echo [glory-quality] No se encontro el npx real fuera del shim. 1>&2
  exit /b 127
)
if not defined GLORY_REAL_NODE (
  for /f "delims=" %%I in ('where node.exe 2^>nul') do if not "%%~fI"=="%~dp0node.cmd" if not defined GLORY_REAL_NODE set "GLORY_REAL_NODE=%%~fI"
)
if not defined GLORY_REAL_NODE (
  echo [glory-quality] No se encontro el node real fuera del shim. 1>&2
  exit /b 127
)
"%GLORY_REAL_NODE%" "%GLORY_GUARD_ROOT%quality-command-guard.mjs" --project-root "%CD%" --executable npx -- %*
if errorlevel 1 exit /b %ERRORLEVEL%
"%GLORY_REAL_NPX%" %*
exit /b %ERRORLEVEL%
