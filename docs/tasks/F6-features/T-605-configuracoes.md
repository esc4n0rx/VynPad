# T-605 — Configurações

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-403, T-504 |
| Status | 🟨 Pronta; falta conferir no app: o vidro ligando e desligando, o UAC do firewall, o reinício e o reinício |

## Objetivo
Tela de preferências no estilo "Ajustes do Sistema" do macOS.

## Escopo
Sub-navegação lateral com as seções:

- **Geral**: nome do server (exibido no celular), iniciar com o sistema, iniciar minimizado, fechar → bandeja, pedir aprovação para novos dispositivos, refazer introdução.
- **Rede**: portas (TCP/UDP, com validação e aviso de reinício do serviço), interface de rede, IPs locais (copiáveis), testar conexão, status do firewall + corrigir.
- **Controles**: status do driver ViGEmBus (versão, reinstalar), perfil padrão para novos dispositivos, taxa de envio sugerida (125/250 Hz), tempo de reserva do slot.
- **Aparência**: tema (Escuro padrão / Claro / Sistema), cor de destaque (paleta macOS + personalizada), transparência (ligada/desligada), reduzir movimento.
- **Idioma**: PT-BR / EN.
- **Privacidade e segurança**: regenerar identidade do server (com confirmação: invalida todos os pareamentos), limpar histórico.
- **Sobre**: versão, verificar atualizações (T-704), licenças de terceiros (incluindo ViGEmBus), abrir pasta de logs, exportar diagnóstico (T-705).

## Critérios de aceite
- [ ] Toda configuração persiste e é aplicada sem reiniciar o app (exceto portas, que reiniciam só o serviço de rede).
- [ ] Busca rápida nas configurações (campo no topo).

## Fora de escopo
- Configurações do mapeamento (T-604).

## Fechamento
- Tela com sub-navegação (Geral, Rede, Controles, Aparência, Idioma, Privacidade e segurança, Sobre) e busca no topo: `settingsIndex.ts` lista todas as configurações, e a busca (sem acentos, várias palavras) mostra só as seções e linhas que casam.
- Backend: `AppSettings` ganhou `server_name`, `close_to_tray`, `default_profile_id`, `send_rate_hz` (125 ou 250), `reservation_secs` (10–120), `transparency` e `reduce_motion`, com validação e testes.
- Aplicado na hora, sem reiniciar: idioma, tema (inclusive "Sistema", que segue o do Windows), cor de destaque, reduzir movimento, transparência (o Mica/Acrylic é tirado e recolocado e a UI recebe `window:chrome-changed`), aprovação de novos dispositivos, perfil padrão dos dispositivos sem perfil próprio, taxa de envio (reenviada aos controles conectados) e tempo de reserva do slot. Portas e nome do server valem na próxima abertura (a tela avisa e oferece "Reiniciar agora").
- Rede: portas validadas como no backend, interfaces com IP copiável, "Testar conexão" (`firewall::self_test`) e firewall com status por porta, "Corrigir" (UAC) e aviso de rede pública.
- Privacidade: "Limpar histórico" e "Regenerar identidade" (gera certificado novo, esquece os dispositivos, encerra as sessões e pede reiniciar), ambos com confirmação.
- Sobre: versões, abrir a pasta de logs, licenças de terceiros (o `THIRD-PARTY-NOTICES.md`, incluindo o ViGEmBus). "Atualizações" (T-704) aparece desabilitado como "Em breve"; "Exportar diagnóstico" e "Abrir a ajuda" vêm da T-705.
- Iniciar com o Windows, iniciar minimizado e fechar → bandeja são gravados aqui e aplicados pela T-607.
