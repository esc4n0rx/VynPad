/**
 * Markdown mínimo para a ajuda do app (os arquivos de `docs/usuario`): títulos (com `{#id}`),
 * parágrafos, listas, blocos de código e, no texto, **negrito**, *itálico*, `código` e links.
 * Não executa nada e não aceita HTML: o que não é reconhecido vira texto.
 */

export type Inline =
  | { type: "text"; text: string }
  | { type: "strong"; children: Inline[] }
  | { type: "em"; children: Inline[] }
  | { type: "code"; text: string }
  | { type: "link"; href: string; children: Inline[] };

export type Block =
  | { type: "heading"; level: 1 | 2 | 3; id: string; children: Inline[]; text: string }
  | { type: "paragraph"; children: Inline[] }
  | { type: "list"; ordered: boolean; items: Inline[][] }
  | { type: "code"; text: string };

const HEADING = /^(#{1,3})\s+(.*?)(?:\s*\{#([A-Za-z0-9_-]+)\})?\s*$/;
const BULLET = /^\s*[-*]\s+(.*)$/;
const NUMBERED = /^\s*\d+\.\s+(.*)$/;

/** Id de um título sem `{#id}`: minúsculas, sem acentos, só letras, números e hífens. */
export function slugify(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** Só âncoras do próprio app (`#id`) e links https viram link. */
function safeHref(href: string): string | null {
  if (href.startsWith("#") || /^https:\/\//i.test(href)) return href;
  return null;
}

/** Negrito, itálico, código e links dentro de uma linha. */
export function parseInline(source: string): Inline[] {
  const out: Inline[] = [];
  let rest = source;
  const push = (text: string) => {
    if (text) out.push({ type: "text", text });
  };
  while (rest.length > 0) {
    const code = /^`([^`]+)`/.exec(rest);
    const strong = /^\*\*(.+?)\*\*/.exec(rest);
    const em = /^\*(?!\s)(.+?)(?<!\s)\*/.exec(rest);
    const link = /^\[([^\]]+)\]\(([^)\s]+)\)/.exec(rest);
    if (code?.[1] !== undefined) {
      out.push({ type: "code", text: code[1] });
      rest = rest.slice(code[0].length);
    } else if (strong?.[1] !== undefined) {
      out.push({ type: "strong", children: parseInline(strong[1]) });
      rest = rest.slice(strong[0].length);
    } else if (em?.[1] !== undefined) {
      out.push({ type: "em", children: parseInline(em[1]) });
      rest = rest.slice(em[0].length);
    } else if (link?.[1] !== undefined && link[2] !== undefined) {
      const href = safeHref(link[2]);
      if (href) out.push({ type: "link", href, children: parseInline(link[1]) });
      else push(link[1]);
      rest = rest.slice(link[0].length);
    } else {
      // Anda até o próximo caractere que pode abrir uma marcação.
      const next = rest.slice(1).search(/[`*[]/);
      const take = next === -1 ? rest.length : next + 1;
      push(rest.slice(0, take));
      rest = rest.slice(take);
    }
  }
  return out;
}

const plain = (inline: Inline[]): string =>
  inline.map((i) => (i.type === "text" || i.type === "code" ? i.text : plain(i.children))).join("");

export function parseMarkdown(source: string): Block[] {
  const lines = source.replace(/\r\n/g, "\n").split("\n");
  const blocks: Block[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i] ?? "";
    if (line.trim() === "") {
      i++;
      continue;
    }
    if (line.startsWith("```")) {
      const code: string[] = [];
      i++;
      while (i < lines.length && !(lines[i] ?? "").startsWith("```")) {
        code.push(lines[i] ?? "");
        i++;
      }
      i++; // fecha o bloco
      blocks.push({ type: "code", text: code.join("\n") });
      continue;
    }
    const heading = HEADING.exec(line);
    if (heading?.[1] !== undefined && heading[2] !== undefined) {
      const children = parseInline(heading[2]);
      const text = plain(children);
      blocks.push({
        type: "heading",
        level: heading[1].length as 1 | 2 | 3,
        id: heading[3] ?? slugify(text),
        children,
        text,
      });
      i++;
      continue;
    }
    const bullet = BULLET.exec(line);
    const numbered = NUMBERED.exec(line);
    if (bullet || numbered) {
      const ordered = numbered !== null && bullet === null;
      const pattern = ordered ? NUMBERED : BULLET;
      const items: Inline[][] = [];
      while (i < lines.length) {
        const match = pattern.exec(lines[i] ?? "");
        if (!match?.[1]) break;
        items.push(parseInline(match[1]));
        i++;
      }
      blocks.push({ type: "list", ordered, items });
      continue;
    }
    // Parágrafo: linhas seguidas até uma em branco ou o começo de outro bloco.
    const paragraph: string[] = [];
    while (i < lines.length) {
      const current = lines[i] ?? "";
      if (
        current.trim() === "" ||
        current.startsWith("```") ||
        HEADING.test(current) ||
        BULLET.test(current) ||
        NUMBERED.test(current)
      ) {
        break;
      }
      paragraph.push(current.trim());
      i++;
    }
    blocks.push({ type: "paragraph", children: parseInline(paragraph.join(" ")) });
  }
  return blocks;
}

/** Os ids dos títulos de um documento (para conferir que um link aponta para algo que existe). */
export function headingIds(source: string): string[] {
  return parseMarkdown(source).flatMap((b) => (b.type === "heading" ? [b.id] : []));
}
