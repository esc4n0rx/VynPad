export const THEME_KEY = "vynpad-theme";

/**
 * Aplica o tema antes da primeira pintura, para a página não piscar no tema errado.
 * "Automático" (ou nada guardado) segue o sistema; dark é o padrão.
 */
const script = `(function(){try{var t=localStorage.getItem("${THEME_KEY}");var d=(t==="dark"||t==="light")?t:(matchMedia("(prefers-color-scheme: light)").matches?"light":"dark");document.documentElement.dataset.theme=d;}catch(e){}})();`;

export function ThemeScript() {
  // Script estático, sem entrada do usuário.
  return <script dangerouslySetInnerHTML={{ __html: script }} />;
}
