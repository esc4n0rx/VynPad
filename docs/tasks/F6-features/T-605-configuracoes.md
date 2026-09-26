# T-605 — Configurações

| Campo | Valor |
|---|---|
| Fase | F6 — Features |
| Prioridade | P1 |
| Estimativa | M |
| Depende de | T-403, T-504 |
| Status | ⬜ A fazer |

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
