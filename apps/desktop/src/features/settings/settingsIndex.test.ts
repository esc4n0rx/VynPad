import { describe, expect, it } from "vitest";
import { i18n } from "@/i18n";
import {
  ITEMS,
  SECTIONS,
  itemHintKey,
  itemLabelKey,
  matchingItems,
  normalize,
} from "./settingsIndex";
import { portProblem } from "./portRules";

const textOf = (id: (typeof ITEMS)[number]["id"]) =>
  `${i18n.t(itemLabelKey(id), { ns: "settings" })} ${i18n.t(itemHintKey(id), { ns: "settings" })}`;

describe("índice das configurações", () => {
  it("cada configuração tem rótulo e dica nos dois idiomas, e uma seção que existe", () => {
    expect(new Set(ITEMS.map((i) => i.id)).size).toBe(ITEMS.length);
    for (const item of ITEMS) {
      expect(SECTIONS).toContain(item.section);
      for (const lng of ["pt-BR", "en"]) {
        for (const key of [itemLabelKey(item.id), itemHintKey(item.id)]) {
          expect(i18n.exists(key, { ns: "settings", lng }), `${key} em ${lng}`).toBe(true);
        }
      }
    }
    for (const section of SECTIONS) {
      expect(
        ITEMS.some((i) => i.section === section),
        section,
      ).toBe(true);
    }
  });

  it("normalize tira acentos e caixa", () => {
    expect(normalize("  Conexão  ")).toBe("conexao");
    expect(normalize("ÁGUA")).toBe("agua");
  });

  it("busca vazia mostra tudo", () => {
    expect(matchingItems("", textOf).size).toBe(ITEMS.length);
    expect(matchingItems("   ", textOf).size).toBe(ITEMS.length);
  });

  it("acha pelo rótulo, pela dica e sem depender de acento", () => {
    const porta = matchingItems("porta", textOf);
    expect(porta.has("ports")).toBe(true);
    expect(porta.has("theme")).toBe(false);
    expect(matchingItems("bandeja", textOf).has("closeToTray")).toBe(true);
    expect(matchingItems("INTRODUCAO", textOf).has("redoIntro")).toBe(true);
  });

  it("várias palavras precisam aparecer todas", () => {
    const hits = matchingItems("firewall regras", textOf);
    expect(hits.has("firewall")).toBe(true);
    expect(hits.size).toBeLessThan(ITEMS.length);
    expect(matchingItems("zzzz", textOf).size).toBe(0);
  });
});

describe("portProblem", () => {
  const ok = { control: 47800, input: 47801, discovery: 47802 };
  it("aceita portas boas", () => {
    expect(portProblem(ok)).toBeNull();
  });
  it("recusa as mesmas coisas que o backend", () => {
    expect(portProblem({ ...ok, control: 80 })).toBe("invalidRange");
    expect(portProblem({ ...ok, input: 70000 })).toBe("invalidRange");
    expect(portProblem({ ...ok, discovery: Number.NaN })).toBe("invalidRange");
    expect(portProblem({ ...ok, input: ok.control })).toBe("duplicate");
    expect(portProblem({ ...ok, discovery: 5353 })).toBe("mdns");
  });
});
