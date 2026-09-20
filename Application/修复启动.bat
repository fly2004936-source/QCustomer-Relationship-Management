@echo off
chcp 936 >nul
setlocal EnableExtensions
title BCRM 客户管理系统 - 启动修复

cd /d "%~dp0"

echo ============================================================
echo    BCRM 客户管理系统 - 启动修复
echo ============================================================
echo.

set "EXE=%~dp0bcrm-desktop.exe"
if not exist "%EXE%" (
  echo [错误] 本目录下找不到 bcrm-desktop.exe
  echo        请把「修复启动.bat」和 bcrm-desktop.exe 放在同一个文件夹里。
  echo.
  pause
  exit /b 1
)

rem ==================================================================
rem  1/5  先判断：程序到底能不能被系统创建出来？
rem       做法：删掉旧报告，跑一次静默自检，看报告有没有重新生成。
rem       报告没生成 = 进程根本没被创建 = 被终端安全软件拦了。
rem       （这一步同时把数据库自检报告也生成好，一箭双雕。）
rem ==================================================================
echo [1/5] 探测程序能否启动（排查是否被终端安全软件拦截）...
if exist "%~dp0bcrm-db-check.txt" del /q "%~dp0bcrm-db-check.txt" >nul 2>&1
"%EXE%" --db-check --quiet

if exist "%~dp0bcrm-db-check.txt" (
  echo        正常：程序可以被创建并运行。
  goto :step2
)

rem ---- 到这里说明 exe 连启动都没启动起来 ----
echo.
echo        ============================================================
echo        [发现] 程序完全没有启动起来 —— 连自检报告都没生成。
echo        ============================================================
echo.
echo        程序启动的第一件事就是写日志、生成报告。这些文件一个都没有，
echo        说明进程根本没被创建。最常见的原因不是程序本身，而是：
echo.
echo          **企业统一安装的终端安全软件把它当成「陌生软件」拦掉了**
echo.
echo        涉及品牌：奇安信天擎、深信服 EDR、360 天擎、CrowdStrike 等。
echo        本程序没有代码签名证书，在受管控的公司电脑上很容易触发。
echo        已知的奇安信天擎拦截策略名：CloudRule.Block.Strangesoftware
echo.
echo        ---- 自动搜索奇安信天擎的拦截日志 ----
echo.
powershell -NoProfile -Command "$d = Join-Path ${env:ProgramFiles(x86)} 'Qianxin\Tianqing\ifl\fcEng\defender\xdlog'; if (Test-Path -LiteralPath $d) { Write-Host '        日志目录：'; Write-Host ('        ' + $d); $h = Get-ChildItem -LiteralPath $d -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 40 | Where-Object { ([Text.Encoding]::Unicode.GetString([IO.File]::ReadAllBytes($_.FullName))) -match 'bcrm' }; if ($h) { Write-Host ''; Write-Host '        >> 命中！以下天擎日志文件里有本程序的拦截记录：' -ForegroundColor Red; $h | Select-Object -First 5 -ExpandProperty Name | ForEach-Object { Write-Host ('           ' + $_) -ForegroundColor Red } } else { Write-Host '        （该目录下未搜到 bcrm 相关记录，但也可能是别的安全软件拦的）' } } else { Write-Host '        （本机没有该目录，可能未安装天擎，或装在别的位置）' }"
echo.
echo        ---- 怎么解决（按推荐顺序）----
echo.
echo        ①【推荐】把程序加入信任 / 白名单
echo           · 奇安信天擎：点右下角托盘图标打开客户端，找「病毒防护」或
echo             「程序管控 / 应用管控」-「信任区 / 白名单」，把这两个文件
echo             加进去（选「文件」类型，填完整路径）：
echo                %~dp0bcrm-desktop.exe
echo                %~dp0BCRM客户管理系统_1.0.0_x64-setup.exe
echo           · 之前若弹过「拦截陌生软件」的窗口，窗口上一般有
echo             「信任 / 允许」按钮，直接点它也能放行。
echo           · 公司统一管控的电脑上，客户端的白名单可能是灰的、改不了。
echo             这种情况要让 IT 在**天擎管理控制台**下发「信任区」策略，
echo             把上面两个文件名和 MD5 提供给他们即可。
echo.
echo        ②【临时可用】右键 exe -「以管理员身份运行」
echo           实测被拦截时提权启动能绕过这条策略。只是每次都要多点一步，
echo           属于应急手段。
echo.
echo        ③【根本解决】给程序做数字签名
echo           用企业代码签名证书签名后就不会再被判为「陌生软件」。
echo.
echo        注意：这种情况程序自身无法规避 —— 拦截发生在进程被创建之前，
echo              程序里任何兜底代码都来不及执行。
echo.
pause
exit /b 1

rem ==================================================================
rem  2/5  WebView2 运行时（界面靠它渲染，缺了什么窗口都不会出现）
rem ==================================================================
:step2
echo.
echo [2/5] 检查 Microsoft Edge WebView2 运行时 ...
set "WV2="
for /f "tokens=2,*" %%A in ('reg query "HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv 2^>nul ^| findstr /i "REG_SZ"') do set "WV2=%%B"
if not defined WV2 for /f "tokens=2,*" %%A in ('reg query "HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv 2^>nul ^| findstr /i "REG_SZ"') do set "WV2=%%B"
if not defined WV2 for /f "tokens=2,*" %%A in ('reg query "HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv 2^>nul ^| findstr /i "REG_SZ"') do set "WV2=%%B"

if defined WV2 (
  echo        已安装，版本 %WV2%
  goto :step3
)
echo        未检测到。界面就是这个组件渲染的，缺了不会有任何窗口。
echo.
echo        即将打开微软官方在线安装页（约 2MB，需要联网）。
echo        装完之后重新双击本程序即可。
echo.
echo        官方下载地址（也可手工复制到浏览器打开）：
echo        https://go.microsoft.com/fwlink/p/?LinkId=2124703
echo.
echo        离线内网机器请到下面这个页面下载 Evergreen 独立安装包：
echo        https://developer.microsoft.com/microsoft-edge/webview2/
echo.
pause
start "" "https://go.microsoft.com/fwlink/p/?LinkId=2124703"
exit /b 1

rem ==================================================================
rem  3/5  WebView2 用户数据目录
rem       被「管理员身份」创建过时，普通用户双击 WebView2 接管不了它，
rem       现象是「进程起来了但没有窗口」，跟缺运行时长得一样。
rem       该目录只是浏览器缓存，删掉会自动重建，不影响 crm.db 业务数据。
rem
rem       程序本身也会在建窗口前自动体检并自愈；这一步是给用户一个
rem       看得见的体检结果，以及删除失败时的手工出口。
rem ==================================================================
:step3
echo.
echo [3/5] 检查 WebView2 用户数据目录 ...
echo.
echo        新版本用的位置（按优先级，二选一）：
call :checkdir "%~dp0.bcrm-webview"               "exe 同级（便携）"
call :checkdir "%LOCALAPPDATA%\Bcrm\WebView2"     "用户数据目录"
echo.
echo        旧版本残留位置（新版本已不再使用，可放心删除）：
call :checkdir "%LOCALAPPDATA%\com.bcrm.desktop"  "旧版默认位"

rem ==================================================================
rem  4/5  数据库自检报告（第 1 步已经生成好了，这里只做展示）
rem ==================================================================
:step4
echo.
echo [4/5] 数据库权限自检 ...
if exist "%~dp0bcrm-db-check.txt" (
  echo        报告已保存：%~dp0bcrm-db-check.txt
  echo        内容摘要（完整内容见该文件）：
  findstr /b /c:"  选用" /c:"  写探针" /c:"  表白名单" /c:"  数据行数" /c:"结论" "%~dp0bcrm-db-check.txt"
) else (
  echo        [注意] 没找到报告文件。请回到第 1 步看提示。
)

rem ==================================================================
rem  5/5  启动
rem ==================================================================
echo.
echo [5/5] 启动 BCRM 客户管理系统 ...
start "" "%EXE%"
timeout /t 3 >nul

echo.
echo 已发起启动。
echo.
echo 如果窗口始终没有出现，请把程序目录下这几个文件发给技术同事：
echo   bcrm-launcher.log         启动过程，最后一行就是卡在哪
echo   bcrm-launcher-error.log   本次致命错误的完整说明
echo   bcrm-db-check.txt         数据库权限自检报告
echo.
echo 另外：请【直接双击】启动，不要右键「以管理员身份运行」——
echo       用管理员身份跑过之后，WebView2 缓存目录会变成管理员所有。
echo       新版本已能自动修复这种情况，但养成直接双击的习惯更省事。
echo.
pause
exit /b 0

rem ------------------------------------------------------------------
rem  子过程：体检一个 WebView2 用户数据目录
rem    参数1 = 目录路径   参数2 = 说明
rem    注意：不要用 goto :eof 之外的方式跳出，否则会打乱主流程
rem ------------------------------------------------------------------
:checkdir
set "D=%~1"
set "N=%~2"
if not defined D goto :eof
if not exist "%D%" (
  echo          [%N%] 不存在
  goto :eof
)
set "OWN="
for /f "usebackq delims=" %%O in (`powershell -NoProfile -NonInteractive -Command "(Get-Acl -LiteralPath '%D%').Owner" 2^>nul`) do set "OWN=%%O"
echo          [%N%] %D%
if defined OWN (echo                 所有者      ：%OWN%) else (echo                 所有者      ：(取不到))
> "%D%\.bcrm_probe" echo x 2>nul
if exist "%D%\.bcrm_probe" (
  del "%D%\.bcrm_probe" >nul 2>&1
  echo                 当前用户可写：是
) else (
  echo                 当前用户可写：否
)
set "GO="
set /p "GO=                 要删除它让它重新生成吗？(仅浏览器缓存，不影响数据) [y/N] "
if /i "%GO%"=="y" (
  rmdir /s /q "%D%"
  if exist "%D%" (echo                 [失败] 仍存在，可能有窗口开着，请关掉后重试。) else (echo                 已删除，下次启动会以当前用户身份重建。)
)
goto :eof
