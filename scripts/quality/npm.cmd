@echo off
setlocal
set "GLORY_GUARD_ROOT=%~dp0"
if not defined GLORY_REAL_NPM (
  for /f "delims=" %%I in ('where npm.cmd 2^>nul') do if /I not "%%~fI"=="%~f0" if not defined GLORY_REAL_NPM set "GLORY_REAL_NPM=%%~fI"
)
if not defined GLORY_REAL_NPM (
  echo [glory-quality] No se encontro el npm real fuera del shim. 1>&2
  exit /b 127
)
node "%GLORY_GUARD_ROOT%quality-command-guard.mjs" --project-root "%CD%" --executable npm -- %*
if errorlevel 1 exit /b %ERRORLEVEL%
"%GLORY_REAL_NPM%" %*
exit /b %ERRORLEVEL%
