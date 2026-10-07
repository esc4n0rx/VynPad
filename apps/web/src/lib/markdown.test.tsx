import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { renderMarkdown, safeHref } from "./markdown";

const html = (md: string) => renderToStaticMarkup(<>{renderMarkdown(md)}</>);

describe("markdown seguro das notas", () => {
  it("renderiza títulos, listas, negrito e código", () => {
    const out = html("## Novidades\n\n- **gyro** por inclinação\n- usa `UDP`\n\nTexto solto.");
    expect(out).toContain("<h4");
    expect(out).toContain("<ul");
    expect(out).toContain("<strong>gyro</strong>");
    expect(out).toContain("<code");
    expect(out).toContain("<p>Texto solto.</p>");
  });

  it("nunca interpreta HTML cru: ele sai escapado", () => {
    const out = html('<script>alert(1)</script> <img src=x onerror="alert(1)">');
    expect(out).not.toContain("<script");
    expect(out).not.toContain("<img");
    expect(out).toContain("&lt;script&gt;");
  });

  it("só links http(s) viram link, com rel de segurança", () => {
    const ok = html("[site](https://vynpad.app/x)");
    expect(ok).toContain('href="https://vynpad.app/x"');
    expect(ok).toContain("noopener");
    const bad = html("[clique](javascript:alert(1)) e [data](data:text/html;base64,AAAA)");
    expect(bad).not.toContain("href=");
    expect(bad).toContain("clique");
  });

  it("safeHref recusa esquemas perigosos e lixo", () => {
    expect(safeHref("https://github.com/a/b")).toBeDefined();
    expect(safeHref("javascript:alert(1)")).toBeUndefined();
    expect(safeHref("//evil.example")).toBeUndefined();
    expect(safeHref("não é url")).toBeUndefined();
  });

  it("texto vazio não quebra", () => {
    expect(html("")).toBe('<div class="space-y-3"></div>');
  });
});
