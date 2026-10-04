# Abre o app com a depuração remota do WebView2 ligada, como o msedgedriver faz, e mostra se a
# porta responde, onde ficou o DevToolsActivePort e que processos o WebView2 criou.
$ErrorActionPreference = "Continue"
$exe = Join-Path (Resolve-Path "$PSScriptRoot\..\..") "target\release\vynpad-desktop.exe"
$data = Join-Path $env:RUNNER_TEMP "vynpad-diag-data"
$udf = Join-Path $env:RUNNER_TEMP "vynpad-diag-webview"
New-Item -ItemType Directory -Force $data, $udf | Out-Null

"== versões"
(Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv
Get-ChildItem "${env:ProgramFiles(x86)}\Microsoft\EdgeWebView\Application" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name

"== políticas do Edge e do WebView2 (registro)"
foreach ($key in 'HKLM:\SOFTWARE\Policies\Microsoft\Edge', 'HKLM:\SOFTWARE\Policies\Microsoft\EdgeWebView', 'HKLM:\SOFTWARE\Policies\Microsoft\Edge\WebView2', 'HKCU:\SOFTWARE\Policies\Microsoft\Edge', 'HKLM:\SOFTWARE\Policies\Google\Chrome') {
  if (Test-Path $key) { "[$key]"; Get-ItemProperty $key | Format-List | Out-String } else { "(sem $key)" }
}
"== variáveis WEBVIEW2 / EDGE do ambiente"
Get-ChildItem Env: | Where-Object { $_.Name -match 'WEBVIEW2|EDGE|CHROME' } | Format-Table -AutoSize | Out-String

"== processos do WebView2 ANTES"
Get-Process msedgewebview2 -ErrorAction SilentlyContinue | Select-Object Id, StartTime | Format-Table | Out-String

$env:VYNPAD_E2E = "1"
$env:VYNPAD_DATA_DIR = $data
$env:WEBVIEW2_USER_DATA_FOLDER = $udf
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"

"== abrindo $exe"
$app = Start-Process $exe -PassThru
Start-Sleep -Seconds 15
"app: " + ($(if ($app.HasExited) { "saiu com código $($app.ExitCode)" } else { "aberto (PID $($app.Id))" }))

"== porta 9222"
try {
  (Invoke-RestMethod http://127.0.0.1:9222/json/version -TimeoutSec 5) | ConvertTo-Json -Depth 3
} catch { "sem resposta: $($_.Exception.Message)" }

"== DevToolsActivePort"
Get-ChildItem $udf -Recurse -Filter DevToolsActivePort -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName; Get-Content $_.FullName }
"(procurando em todo o LOCALAPPDATA)"
Get-ChildItem $env:LOCALAPPDATA -Recurse -Filter DevToolsActivePort -ErrorAction SilentlyContinue | Select-Object -First 5 FullName

"== processos do WebView2 DEPOIS"
Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" |
  Where-Object { $_.CommandLine -notmatch "--type=" } |
  ForEach-Object { "PID $($_.ProcessId): $($_.CommandLine)" }

"== log do app"
Get-ChildItem "$env:LOCALAPPDATA\VynPad\logs" -ErrorAction SilentlyContinue | ForEach-Object { "--- $($_.Name)"; Get-Content $_.FullName -Tail 20 }

Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue
Get-Process msedgewebview2 -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
