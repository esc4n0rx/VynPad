import { createElement, type ReactNode } from "react";
import type { Block, Inline } from "@/lib/markdown";

interface MarkdownProps {
  blocks: readonly Block[];
  /** Clique num link `#id` do próprio documento (ou de outro): quem usa decide para onde ir. */
  onAnchor: (id: string) => void;
}

function renderInline(items: readonly Inline[], onAnchor: (id: string) => void): ReactNode {
  return items.map((item, index) => {
    switch (item.type) {
      case "text":
        return item.text;
      case "code":
        return (
          <code key={index} className="rounded-sm bg-control px-1 py-0.5 text-[0.92em]">
            {item.text}
          </code>
        );
      case "strong":
        return (
          <strong key={index} className="font-semibold text-fg">
            {renderInline(item.children, onAnchor)}
          </strong>
        );
      case "em":
        return <em key={index}>{renderInline(item.children, onAnchor)}</em>;
      case "link":
        return item.href.startsWith("#") ? (
          <a
            key={index}
            href={item.href}
            onClick={(event) => {
              event.preventDefault();
              onAnchor(item.href.slice(1));
            }}
            className="rounded-sm text-accent underline-offset-2 focus-ring hover:underline"
          >
            {renderInline(item.children, onAnchor)}
          </a>
        ) : (
          // Links externos só chegam aqui se forem https (ver `parseInline`).
          <a
            key={index}
            href={item.href}
            target="_blank"
            rel="noreferrer noopener"
            className="rounded-sm text-accent underline-offset-2 focus-ring hover:underline"
          >
            {renderInline(item.children, onAnchor)}
          </a>
        );
    }
  });
}

const HEADING_CLASS = {
  1: "mb-3 text-title font-semibold tracking-tight text-fg",
  2: "mt-6 mb-2 text-subtitle font-semibold text-fg scroll-mt-4",
  3: "mt-4 mb-1 text-body font-semibold text-fg scroll-mt-4",
} as const;

/** Desenha os blocos de `parseMarkdown` com o visual do app. */
export function Markdown({ blocks, onAnchor }: MarkdownProps) {
  return (
    <div className="flex flex-col gap-2 text-body text-fg-2">
      {blocks.map((block, index) => {
        switch (block.type) {
          case "heading":
            return createElement(
              `h${String(block.level)}`,
              { key: index, id: block.id, className: HEADING_CLASS[block.level] },
              renderInline(block.children, onAnchor),
            );
          case "paragraph":
            return <p key={index}>{renderInline(block.children, onAnchor)}</p>;
          case "list": {
            const Tag = block.ordered ? "ol" : "ul";
            return (
              <Tag
                key={index}
                className={`flex flex-col gap-1 pl-5 ${block.ordered ? "list-decimal" : "list-disc"}`}
              >
                {block.items.map((item, i) => (
                  <li key={i}>{renderInline(item, onAnchor)}</li>
                ))}
              </Tag>
            );
          }
          case "code":
            return (
              <pre
                key={index}
                className="overflow-auto rounded-lg bg-control p-3 text-caption text-fg select-text"
              >
                <code>{block.text}</code>
              </pre>
            );
        }
      })}
    </div>
  );
}
