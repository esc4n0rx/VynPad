import { describe, expect, it } from "vitest";
import { fmt, negotiateLocale } from "./config";
import { en } from "./en";
import { ptBR } from "./pt-BR";

/** Todas as chaves de um objeto aninhado, no formato `a.b.0.c`. */
function keys(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) return [prefix];
  return Object.entries(value).flatMap(([k, v]) => keys(v, prefix ? `${prefix}.${k}` : k));
}

describe("idiomas", () => {
  it("pt-BR e en têm exatamente as mesmas chaves e a mesma quantidade de itens", () => {
    expect(keys(en)).toEqual(keys(ptBR));
  });

  it("nenhum texto está vazio e os marcadores {x} batem entre os idiomas", () => {
    const pt = new Map(keys(ptBR).map((k) => [k, get(ptBR, k)]));
    for (const k of keys(en)) {
      const english = get(en, k);
      const portuguese = pt.get(k);
      expect(english.trim(), k).not.toBe("");
      expect(portuguese?.trim(), k).not.toBe("");
      const marks = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join();
      expect(marks(english), k).toBe(marks(portuguese ?? ""));
    }
  });
});

function get(root: unknown, path: string): string {
  let cursor = root;
  for (const part of path.split(".")) cursor = (cursor as Record<string, unknown>)[part];
  return cursor as string;
}

describe("negociação de idioma", () => {
  it("o cookie vence o navegador", () => {
    expect(negotiateLocale("pt-BR,pt;q=0.9", "en")).toBe("en");
  });

  it("cookie inválido é ignorado", () => {
    expect(negotiateLocale("en-US,en;q=0.9", "fr")).toBe("en");
  });

  it("respeita o peso q e a ordem", () => {
    expect(negotiateLocale("en;q=0.5, pt-PT;q=0.8")).toBe("pt-BR");
    expect(negotiateLocale("fr-FR,fr;q=0.9,en-GB;q=0.8,en;q=0.7")).toBe("en");
    expect(negotiateLocale("en;q=0, pt;q=0.1")).toBe("pt-BR");
  });

  it("idioma desconhecido ou cabeçalho ausente cai no padrão", () => {
    expect(negotiateLocale("ja,zh;q=0.9")).toBe("pt-BR");
    expect(negotiateLocale(null)).toBe("pt-BR");
    expect(negotiateLocale("*")).toBe("pt-BR");
  });
});

describe("fmt", () => {
  it("troca os marcadores e deixa os desconhecidos como estão", () => {
    expect(fmt("Versão {v} para {os}", { v: "1.0", os: "Windows" })).toBe(
      "Versão 1.0 para Windows",
    );
    expect(fmt("{a} e {b}", { a: 1 })).toBe("1 e {b}");
  });
});
