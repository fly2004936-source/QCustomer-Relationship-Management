@echo off
chcp 936 >nul
setlocal enabledelayedexpansion
title BCRM 客户管理系统 · Web 版

rem ============================================================
rem  BCRM 客户管理系统 · Web 版一键启动
rem
rem  双击本文件即可：
rem    1. 找到后端程序与前端页面产物
rem    2. 起一个服务（接口和页面在同一个进程、同一个端口）
rem    3. 自动打开浏览器
rem
rem  关闭本窗口 = 停止服务。
rem
rem  可选参数：
rem    启动Web版.bat                 默认 127.0.0.1:8787，仅本机可访问
rem    启动Web版.bat 9000            改用 9000 端口
rem    启动Web版.bat 9000 0.0.0.0    监听所有网卡（局域网可访问）
rem
rem  注意：当前后端接口没有任何鉴权，用 0.0.0.0 之前请确认网络边界，
rem        不要在能上外网的机器上直接暴露。
rem ============================================================

set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"

set "PORT=%~1"
if not defined PORT set "PORT=8787"
set "HOST=%~2"
if not defined HOST set "HOST=127.0.0.1"

set "DATADIR=%ROOT%\data"
set "DB=%DATADIR%\crm.db"
set "LOG=%DATADIR%\web-server.log"
set "WEBDIR="
set "BACKEND="

echo.
echo ============================================================
echo   BCRM 客户管理系统 · Web 版
echo ============================================================
echo.

rem ---------- 0. 已经在跑就别再起第二个 ----------
tasklist /fi "imagename eq bcrm-backend.exe" 2>nul | findstr /i "bcrm-backend.exe" >nul
if not errorlevel 1 (
  echo   检测到 BCRM 服务已经在运行，直接打开浏览器。
  echo.
  echo   如果打开的是空白页，说明上一次的服务用的不是 %PORT% 端口，
  echo   请先关掉原来那个窗口，再双击本文件。
  echo.
  start "" "http://127.0.0.1:%PORT%/"
  pause >nul
  exit /b 0
)

rem ---------- 1. 定位后端程序 ----------
rem  注意：exe 必须放在静态托管目录「之外」。
rem  如果把 exe 塞进 web\ 里，它自己也会被 HTTP 当成静态文件下载走。
for %%P in (
  "%ROOT%\bcrm-backend.exe"
  "%ROOT%\Application\bcrm-backend.exe"
  "%ROOT%\backend\target\release\bcrm-backend.exe"
  "%ROOT%\bcrm-server.exe"
) do (
  if not defined BACKEND if exist %%P set "BACKEND=%%~fP"
)

rem ---------- 2. 定位前端页面产物 ----------
for %%P in (
  "%ROOT%\web\index.html"
  "%ROOT%\Application\web\index.html"
  "%ROOT%\frontend\dist\index.html"
) do (
  if not defined WEBDIR if exist %%P set "WEBDIR=%%~dpP"
)

rem 【注意】必须去掉尾部的反斜杠 —— %%~dpP 返回的「盘符+路径」结尾是带 \ 的。
rem 若原样放进引号里，命令行会变成：
rem     --web-dir "C:\...\frontend\dist\" --db "C:\...\crm.db" --port 8787
rem 那个 \" 在 Windows 命令行解析里是「字面双引号」，而不是「引号结束」，
rem 于是引号一直不闭合，后面的 --db / --port / --host 全被吞进 --web-dir 的值里。
rem 实测症状（日志原文）：
rem     --web-dir 指向的目录里没有 index.html：
rem     C:\...\frontend\dist" --db C:\...\crm.db --port 8787 --host 127.0.0.1
if defined WEBDIR if "%WEBDIR:~-1%"=="\" set "WEBDIR=%WEBDIR:~0,-1%"

if not defined BACKEND (
  echo   [错误] 找不到后端程序 bcrm-backend.exe
  echo.
  echo   已查找：
  echo     bcrm-backend.exe
  echo     Application\bcrm-backend.exe
  echo     backend\target\release\bcrm-backend.exe
  echo.
  echo   请先编译后端：
  echo     cargo build --release --manifest-path backend\Cargo.toml
  echo.
  pause
  exit /b 1
)

if not defined WEBDIR (
  echo   [错误] 找不到前端页面产物 index.html
  echo.
  echo   已查找：
  echo     web\index.html
  echo     frontend\dist\index.html
  echo.
  echo   请先构建前端：
  echo     cd frontend
  echo     npm install
  echo     npm run build
  echo.
  pause
  exit /b 1
)

rem ---------- 3. 准备数据目录 ----------
if not exist "%DATADIR%" mkdir "%DATADIR%" >nul 2>&1

rem ---------- 4. 选一个空闲端口 ----------
set /a MAXPORT=%PORT%+20
:portloop
netstat -ano | findstr /c:":%PORT% " | findstr /i "LISTENING" >nul 2>&1
if errorlevel 1 goto :portok
set /a PORT+=1
if !PORT! geq %MAXPORT% (
  echo   [错误] 从起始端口起连续 20 个端口都被占用。
  echo          请手动指定一个，例如：启动Web版.bat 9000
  echo.
  pause
  exit /b 1
)
goto :portloop
:portok

echo   后端程序 : %BACKEND%
echo   前端页面 : %WEBDIR%
echo   数据库   : %DB%
echo   监听地址 : %HOST%
echo   访问地址 : http://127.0.0.1:%PORT%/
echo.

rem ---------- 5. 启动服务（接口 + 页面同一个进程） ----------
echo   正在启动服务 ...
start "" /b "%BACKEND%" --web-dir "%WEBDIR%" --db "%DB%" --port %PORT% --host %HOST% 1>"%LOG%" 2>&1

rem ---------- 6. 等待就绪 ----------
set /a TRY=0
:wait
set /a TRY+=1
powershell -NoProfile -Command "try{$c=New-Object Net.Sockets.TcpClient;$c.Connect('127.0.0.1',%PORT%);$c.Close();exit 0}catch{exit 1}" >nul 2>&1
if not errorlevel 1 goto :ready
if !TRY! geq 40 goto :failed
ping -n 2 127.0.0.1 >nul
goto :wait

:ready
echo   [OK] 服务已就绪，正在打开浏览器 ...
echo.
start "" "http://127.0.0.1:%PORT%/"
echo ------------------------------------------------------------
echo   界面地址 : http://127.0.0.1:%PORT%/
echo   接口地址 : http://127.0.0.1:%PORT%/api/v1
echo   运行日志 : %LOG%
echo ------------------------------------------------------------
echo.
echo   关闭本窗口即可停止服务。
echo.
pause >nul
goto :done

:failed
echo   [错误] 服务在 40 秒内没有启动成功。
echo.
echo   ---- 服务端最后的输出 ----
if exist "%LOG%" goto :showlog
echo   ^(日志文件都没生成 —— 进程很可能压根没被创建，见下面第 1 条^)
goto :logdone
:showlog
type "%LOG%"
:logdone
echo   --------------------------
echo.
echo   日志文件：%LOG%
echo.
echo   常见原因：
echo     1. 被安全软件在「创建进程」阶段拦截（奇安信天擎等）
echo        症状：程序目录里连日志文件都没有生成。
echo        解法：把本目录加进终端管控的信任区（按目录加，不是按文件）。
echo     2. 数据库文件被别的程序占用，或当前目录没有写权限。
echo     3. 端口被防火墙 / 策略拦截。
echo.
pause
exit /b 1

:done
rem 服务与当前窗口共享同一个控制台，窗口关闭时随之结束
exit /b 0
