; Ganchos do instalador NSIS do Tauri (T-703). Os macros abaixo são chamados pelo template padrão.
;
; Instalação: nada além do padrão. As regras de firewall e o ViGEmBus são oferecidos pelo
; onboarding do app, com consentimento (D-11), e não pelo instalador.
;
; Desinstalação: remove o que o app criou fora da pasta de instalação, para não deixar sobras:
;   - regras de firewall "VynPad *" (precisam de administrador, então pedem UAC só se existirem);
;   - a entrada de "Iniciar com o Windows" (HKCU\...\Run);
;   - os dados (banco, perfis, logs) somente se o usuário marcar a opção do desinstalador.

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Desinstalação silenciosa (/S) nunca abre UAC: as regras ficam e o usuário pode removê-las depois.
  IfSilent vynpad_skip_firewall 0
  ; Sai com o número de regras encontradas; consultar não exige administrador.
  nsExec::Exec `powershell.exe -NoProfile -NonInteractive -Command "exit @(Get-NetFirewallRule -DisplayName 'VynPad *' -ErrorAction SilentlyContinue).Count"`
  Pop $0
  ${If} $0 != 0
    ExecShellWait "runas" "powershell.exe" `-NoProfile -WindowStyle Hidden -Command "Remove-NetFirewallRule -DisplayName 'VynPad *'"` SW_HIDE
  ${EndIf}
  vynpad_skip_firewall:

  ; Iniciar com o Windows: a chave e o "aprovado" que o Gerenciador de Tarefas guarda.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "VynPad"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "VynPad"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; O app guarda os dados em %APPDATA%\VynPad e os logs em %LOCALAPPDATA%\VynPad; o
  ; desinstalador do Tauri só conhece as pastas com o identificador do app.
  ${If} $DeleteAppDataCheckboxState == 1
    RMDir /r "$APPDATA\VynPad"
    RMDir /r "$LOCALAPPDATA\VynPad"
  ${EndIf}
!macroend
