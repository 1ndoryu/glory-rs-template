@echo off
setlocal
set "GLORY_GUARD_ROOT=%~dp0"
set "GLORY_REAL_CARGO=%GLORY_REAL_CARGO%"
if not defined GLORY_REAL_CARGO set "GLORY_REAL_CARGO=C:\Users\Owner\.cargo\bin\cargo.exe"
node "%GLORY_GUARD_ROOT%heavy-run-guard.mjs" --execute-cargo --project-root "%CD%" --cargo-path "%GLORY_REAL_CARGO%" -- %*
exit /b %ERRORLEVEL%
