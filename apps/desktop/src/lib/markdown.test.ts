import { describe, expect, it } from "vitest";
import { headingIds, parseInline, parseMarkdown, slugify } from "./markdown";

describe("parseInline", () => {
  it("reconhece negrito, itálico, código e links", () => {
    expect(parseInline("a **b** c *d* `e` [f](#g)")).toEqual([
      { type: "text", text: "a " },
      { type: "strong", children: [{ type: "text", text: "b" }] },
      { type: "text", text: " c " },
      { type: "em", children: [{ type: "text", text: "d" }] },
      { type: "text", text: " " },
      { type: "code", text: "e" },
      { type: "text", text: " " },
      { type: "link", href: "#g", children: [{ type: "text", text: "f" }] },
    ]);
  });

  it("só aceita âncoras do app e links https: o resto vira texto", () => {
    for (const bad of [
      "javascript:void",
      "http://exemplo.test",
      "file:///c:/x",
      "data:text/html,x",
    ]) {
      expect(parseInline(`[clique](${bad})`)).toEqual([{ type: "text", text: "clique" }]);
    }
    expect(parseInline("[ok](https://exemplo.test/a)")[0]).toMatchObject({
      type: "link",
      href: "https://exemplo.test/a",
    });
  });

  it("não interpreta HTML nem asteriscos soltos", () => {
    expect(parseInline("<script>x</script> 2 * 3 * 4")).toEqual([
      { type: "text", text: "<script>x</script> 2 " },
      { type: "text", text: "* 3 " },
      { type: "text", text: "* 4" },
    ]);
  });

  it("marcações dentro de marcações funcionam", () => {
    const [strong] = parseInline("**veja `isto`**");
    expect(strong).toMatchObject({ type: "strong" });
  });
});

describe("parseMarkdown", () => {
  it("títulos levam o id explícito ou um gerado do texto", () => {
    const [h1, h2, h3] = parseMarkdown(
      "# Primeiro {#um}\n## Solução de problemas\n### Já está certo {#ok-1}",
    );
    expect(h1).toMatchObject({ type: "heading", level: 1, id: "um", text: "Primeiro" });
    expect(h2).toMatchObject({ level: 2, id: "solucao-de-problemas" });
    expect(h3).toMatchObject({ level: 3, id: "ok-1" });
  });

  it("agrupa linhas em parágrafos e separa por linha em branco", () => {
    const blocks = parseMarkdown("linha um\nlinha dois\n\noutro parágrafo");
    expect(blocks).toHaveLength(2);
    expect(blocks[0]).toEqual({
      type: "paragraph",
      children: [{ type: "text", text: "linha um linha dois" }],
    });
  });

  it("lê listas com marcadores e numeradas", () => {
    const [bullets, numbered] = parseMarkdown("- a\n- b\n\n1. um\n2. dois\n3. três");
    expect(bullets).toMatchObject({ type: "list", ordered: false });
    expect(bullets?.type === "list" && bullets.items).toHaveLength(2);
    expect(numbered).toMatchObject({ type: "list", ordered: true });
    expect(numbered?.type === "list" && numbered.items).toHaveLength(3);
  });

  it("blocos de código ficam literais", () => {
    const [code] = parseMarkdown("```text\n# não é título\ncargo test\n```");
    expect(code).toEqual({ type: "code", text: "# não é título\ncargo test" });
  });

  it("um parágrafo termina quando começa um título ou uma lista", () => {
    const blocks = parseMarkdown("texto\n## Título\n- item");
    expect(blocks.map((b) => b.type)).toEqual(["paragraph", "heading", "list"]);
  });

  it("aceita fim de linha do Windows", () => {
    expect(parseMarkdown("# A {#a}\r\n\r\ntexto\r\n")).toHaveLength(2);
  });

  it("headingIds lista os ids", () => {
    expect(headingIds("# A {#a}\n## B\ntexto\n## C {#c}")).toEqual(["a", "b", "c"]);
  });
});

describe("slugify", () => {
  it("tira acentos e pontuação", () => {
    expect(slugify("Permissão do Windows recusada!")).toBe("permissao-do-windows-recusada");
    expect(slugify("  --Já  vai--  ")).toBe("ja-vai");
  });
});
