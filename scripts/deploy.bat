@echo off
echo.
echo  IceLines — regenerate and deploy
echo  ================================
echo.

cd /d "%~dp0\.."

echo [1/3] Building site with icelines...
src\target\release\icelines.exe build --no-site
if errorlevel 1 (
    echo ERROR: icelines build failed - run: cargo build --release -p icelines-cli
    pause
    exit /b 1
)

echo.
echo [2/3] Deploying to GitHub Pages...
set PYTHONUTF8=1
mkdocs gh-deploy --remote-name origin --force
if errorlevel 1 (
    echo ERROR: mkdocs gh-deploy failed
    pause
    exit /b 1
)

echo.
echo [3/3] Done!
echo  Live at: https://giodl73-repo.github.io/ICELINES/
echo.
pause
