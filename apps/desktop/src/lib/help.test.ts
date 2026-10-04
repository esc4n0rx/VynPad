import { describe, expect, it } from "vitest";
import type { ErrorCode } from "@/lib/errors";
import {
  ERROR_HELP,
  HELP_DOCS,
  docWithHeading,
  helpHashForError,
  helpSource,
  type HelpLanguage,
} from "./help";
import { headingIds, parseMarkdown } from "./markdown";

const LANGUAGES: HelpLanguage[] = ["pt-BR", "en"];

describe("documentos da ajuda", () => {
  it("todos existem nos dois idiomas e começam por um título", () => {
    for (const language of LANGUAGES) {
      for (const doc of HELP_DOCS) {
        const blocks = parseMarkdown(helpSource(doc, language));
        expect(blocks.length, `${language}/${doc}`).toBeGreaterThan(3);
        expect(blocks[0], `${language}/${doc}`).toMatchObject({ type: "heading", level: 1 });
      }
    }
  });

  it("os dois idiomas têm os mesmos títulos (mesmos ids), na mesma ordem", () => {
    for (const doc of HELP_DOCS) {
      expect(headingIds(helpSource(doc, "en")), doc).toEqual(headingIds(helpSource(doc, "pt-BR")));
    }
  });

  it("ids de título não se repetem entre documentos", () => {
    for (const language of LANGUAGES) {
      const all = HELP_DOCS.flatMap((doc) => headingIds(helpSource(doc, language)));
      expect(new Set(all).size, language).toBe(all.length);
    }
  });

  it("todo link interno aponta para um título que existe", () => {
    for (const language of LANGUAGES) {
      for (const doc of HELP_DOCS) {
        const links = [...helpSource(doc, language).matchAll(/\]\(#([A-Za-z0-9_-]+)\)/g)].map(
          (m) => m[1] ?? "",
        );
        for (const id of links) {
          expect(docWithHeading(id, language), `${language}/${doc} → #${id}`).not.toBeNull();
        }
      }
    }
  });
});

describe("ajuda dos erros", () => {
  const codes = Object.keys(ERROR_HELP) as ErrorCode[];

  it("cada erro conhecido da UI tem uma seção na solução de problemas, nos dois idiomas", () => {
    expect(codes.length).toBeGreaterThanOrEqual(9);
    for (const language of LANGUAGES) {
      const ids = headingIds(helpSource("solucao-de-problemas", language));
      for (const code of codes) {
        expect(ids, `${language}: ${code}`).toContain(ERROR_HELP[code]);
        expect(docWithHeading(ERROR_HELP[code], language)).toBe("solucao-de-problemas");
      }
    }
  });

  it("o endereço da ajuda de um erro usa o roteador por hash", () => {
    expect(helpHashForError("no_network")).toBe("#/help?topic=err-no-network");
  });
});
