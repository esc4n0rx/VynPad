import type { ReactNode } from "react";

/**
 * Um renderizador de Markdown mínimo e **seguro** para as notas dos releases (que vêm do GitHub,
 * ou seja, de fora). Só entende o que as notas usam: títulos, listas, parágrafos, `código`,
 * **negrito** e [links](https://...). Tudo vira elemento React (texto escapado pelo React): HTML
 * cru nunca é interpretado e só links `http(s)` viram link.
 */

const INLINE = /(\*\*[^*\n]+\*\*|`[^`\n]+`|\[[^\]\n]+\]\([^)\s]+\))/g;

/** Aceita só `http` e `https`; qualquer outra coisa (javascript:, data:...) fica como texto. */
export function safeHref(raw: string): string | undefined {
  try {
    const url = new URL(raw);
    return url.protocol === "https:" || url.protocol === "http:" ? url.toString() : undefined;
  } catch {
    return undefined;
  }
}

function inline(text: string, keyPrefix: string): ReactNode[] {
  return text.split(INLINE).map((part, index) => {
    const key = `${keyPrefix}-${String(index)}`;
    if (part.startsWith("**") && part.endsWith("**") && part.length > 4) {
      return <strong key={key}>{part.slice(2, -2)}</strong>;
    }
    if (part.startsWith("`") && part.endsWith("`") && part.length > 2) {
      return (
        <code key={key} className="rounded bg-control px-1.5 py-0.5 font-mono text-[0.85em]">
          {part.slice(1, -1)}
        </code>
      );
    }
    const link = /^\[([^\]]+)\]\(([^)\s]+)\)$/.exec(part);
    if (link) {
      const href = safeHref(link[2] ?? "");
      if (href) {
        return (
          <a
            key={key}
            href={href}
            rel="noopener noreferrer nofollow"
            className="text-accent hover:underline"
          >
            {link[1]}
          </a>
        );
      }
      return link[1];
    }
    return part;
  });
}

export function renderMarkdown(source: string): ReactNode {
  const blocks: ReactNode[] = [];
  let list: string[] = [];
  let paragraph: string[] = [];

  const flushList = () => {
    if (list.length === 0) return;
    const items = list;
    list = [];
    const key = `ul-${String(blocks.length)}`;
    blocks.push(
      <ul key={key} className="list-disc space-y-1.5 pl-5">
        {items.map((item, i) => (
          <li key={`${key}-${String(i)}`}>{inline(item, `${key}-${String(i)}`)}</li>
        ))}
      </ul>,
    );
  };
  const flushParagraph = () => {
    if (paragraph.length === 0) return;
    const text = paragraph.join(" ");
    paragraph = [];
    const key = `p-${String(blocks.length)}`;
    blocks.push(<p key={key}>{inline(text, key)}</p>);
  };

  for (const line of source.replace(/\r\n/g, "\n").split("\n")) {
    const trimmed = line.trim();
    const heading = /^(#{1,6})\s+(.+)$/.exec(trimmed);
    const bullet = /^[-*+]\s+(.+)$/.exec(trimmed);
    if (heading) {
      flushList();
      flushParagraph();
      const key = `h-${String(blocks.length)}`;
      blocks.push(
        <h4 key={key} className="mt-2 font-semibold text-fg">
          {inline(heading[2] ?? "", key)}
        </h4>,
      );
    } else if (bullet) {
      flushParagraph();
      list.push(bullet[1] ?? "");
    } else if (trimmed === "") {
      flushList();
      flushParagraph();
    } else {
      flushList();
      paragraph.push(trimmed);
    }
  }
  flushList();
  flushParagraph();
  return <div className="space-y-3">{blocks}</div>;
}
