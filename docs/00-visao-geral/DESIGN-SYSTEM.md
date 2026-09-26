# Design System — estilo macOS Sonoma

## 1. Princípios

1. **Vidro e profundidade**: fundo translúcido (Mica/Acrylic), camadas com blur e sombras suaves, sem bordas duras.
2. **Calma por padrão**: poucos elementos por tela, bastante respiro, cor usada para indicar estado, não para decorar.
3. **Vivo**: o controle na tela reflete o input em tempo real; transições com spring.
4. **Dark first**: dark mode é o padrão; light mode é opcional (Configurações → Aparência).

## 2. Janela

| Item | Especificação |
|---|---|
| Moldura | `decorations: false`, `transparent: true`, `shadow: true` (Tauri 2) |
| Cantos | Win11: arredondamento nativo do DWM (8 px). Win10/Linux: `border-radius: 12px` no container raiz + área transparente |
| Material | Win11: `apply_mica` (dark). Win10: `apply_acrylic` com tint `#1C1C1EB3`. Linux: fundo sólido `--bg-window` |
| Barra de título | 44 px, `data-tauri-drag-region`; **semáforo** à esquerda (fechar #FF5F57, minimizar #FEBC2E, maximizar #28C840), ícones aparecem no hover |
| Sidebar | 220 px, translúcida (`--bg-sidebar`), itens com ícone + label, seleção em pílula com accent 20% |
| Tamanho | padrão 1100×720, mínimo 900×600 |

## 3. Tokens (dark)

```css
:root[data-theme="dark"] {
  --accent: #0A84FF;
  --accent-hover: #409CFF;
  --accent-soft: rgb(10 132 255 / 0.18);

  --bg-window: rgb(28 28 30 / 0.72);   /* sobre Mica */
  --bg-sidebar: rgb(44 44 46 / 0.45);
  --bg-elevated: rgb(58 58 60 / 0.55);  /* cards */
  --bg-control: rgb(118 118 128 / 0.24);
  --separator: rgb(84 84 88 / 0.45);

  --text-primary: rgb(255 255 255 / 0.92);
  --text-secondary: rgb(235 235 245 / 0.60);
  --text-tertiary: rgb(235 235 245 / 0.30);

  --success: #30D158;
  --warning: #FFD60A;
  --danger:  #FF453A;
  --info:    #64D2FF;

  --radius-sm: 6px;  --radius-md: 10px;  --radius-lg: 14px;  --radius-xl: 20px;
  --shadow-card: 0 1px 0 rgb(255 255 255 / 0.06) inset, 0 8px 24px rgb(0 0 0 / 0.35);
  --blur: 24px;
}
```

A paleta light espelha os mesmos tokens (valores do macOS light) e é aplicada com `data-theme="light"`.

### Cores dos slots (identificam cada controle na UI)

| Slot | Cor | Uso |
|---|---|---|
| P1 | `#0A84FF` azul | anel do ícone, LED virtual, gráficos |
| P2 | `#30D158` verde | idem |
| P3 | `#FF9F0A` laranja | idem |
| P4 | `#BF5AF2` roxo | idem |

## 4. Tipografia

- Fonte: **Inter** (fallback: `"Segoe UI Variable", system-ui`), com `font-feature-settings: "cv11", "ss01"`.
- Escala: 11 (caption) · 13 (corpo, padrão macOS) · 15 (subtítulo) · 20 (título) · 28 (hero do onboarding).
- Pesos: 400 / 500 / 600. Números em `tabular-nums` nas métricas.

## 5. Movimento

- Biblioteca: **Motion** (Framer Motion).
- Spring padrão: `{ type: "spring", stiffness: 380, damping: 32 }`.
- Troca de páginas: fade + 8 px de deslocamento em 180 ms.
- Conexão de um controle: o card do slot "acende" (glow na cor do slot) + haptic visual.
- `prefers-reduced-motion`: sem deslocamento, só fade.

## 6. Iconografia

- UI geral: **lucide-react** (stroke 1.75).
- **Gamepad**: SVG próprio do controle X360 (`assets/gamepad/x360.svg`) com cada botão, gatilho e analógico como grupo nomeado (`#btn-a`, `#stick-left`...), para acender e animar com o input ao vivo.
- Ícones de dispositivo: celular com mini-gamepad, variantes `connected` / `idle` / `pairing` / `offline`.
- Ícone do app: gamepad arredondado sobre um squircle com gradiente azul (#0A84FF → #5E5CE6).

## 7. Componentes base (shadcn/ui customizados)

Button (primary, secondary, ghost, destructive) · Toggle estilo macOS · Segmented control · Slider com valor · Select · Card de vidro · Sheet/Modal com blur · Toast · Tooltip · Badge de status · Stepper do onboarding · Empty state com ilustração · Skeleton.

## 8. Telas

| Tela | Conteúdo principal |
|---|---|
| Onboarding | 5 passos: boas-vindas → driver ViGEmBus → firewall → parear primeiro celular (QR) → pronto |
| Dashboard | 4 cards de slot (P1–P4): gamepad ao vivo, nome do dispositivo, bateria, ping, perfil ativo |
| Dispositivos | "Próximos na rede" (auto detect) + "Pareados"; botão "Parear novo" abre o QR/PIN |
| Mapeamento | Gamepad interativo à esquerda; painel de propriedades à direita (botões, sticks, gatilhos, gyro); seletor de perfil |
| Latência | Gráfico de linha por slot (RTT p50/p95), jitter, perda, tempo de processamento no server |
| Configurações | Geral · Rede · Aparência · Idioma · Sobre |
