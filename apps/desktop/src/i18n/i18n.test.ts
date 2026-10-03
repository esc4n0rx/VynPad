import { afterEach, describe, expect, it } from "vitest";
import en from "./en.json";
import { formatDateTime, formatNumber } from "./format";
import { messageForError } from "./errors";
import { i18n, pickLanguage, setLanguage } from "./index";
import ptBR from "./pt-BR.json";

/** Todas as chaves folha de um arquivo de idioma, como `common.nav.label`. */
function leafKeys(node: unknown, prefix = ""): string[] {
  if (typeof node !== "object" || node === null) return [prefix];
  return Object.entries(node).flatMap(([key, value]) =>
    leafKeys(value, prefix ? `${prefix}.${key}` : key),
  );
}

/** Placeholders `{{nome}}` de cada texto: PT e EN precisam usar os mesmos. */
function placeholders(node: unknown, prefix = ""): Record<string, string[]> {
  if (typeof node === "string") {
    return { [prefix]: [...node.matchAll(/\{\{(\w+)\}\}/g)].map((m) => m[1] ?? "").sort() };
  }
  return Object.entries(node as object).reduce<Record<string, string[]>>(
    (acc, [key, value]) => ({ ...acc, ...placeholders(value, prefix ? `${prefix}.${key}` : key) }),
    {},
  );
}

afterEach(async () => {
  await setLanguage("pt-BR");
});

describe("arquivos de idioma", () => {
  it("PT-BR e EN têm exatamente as mesmas chaves", () => {
    const pt = leafKeys(ptBR).sort();
    const english = leafKeys(en).sort();
    expect(
      english.filter((k) => !pt.includes(k)),
      "só em EN",
    ).toEqual([]);
    expect(
      pt.filter((k) => !english.includes(k)),
      "só em PT-BR",
    ).toEqual([]);
  });

  it("os dois idiomas usam os mesmos placeholders em cada texto", () => {
    expect(placeholders(en)).toEqual(placeholders(ptBR));
  });

  it("nenhum texto está vazio", () => {
    for (const file of [ptBR, en]) {
      const all = Object.entries(placeholders(file)).map(([key]) => key);
      expect(all.length).toBeGreaterThan(0);
    }
    expect(JSON.stringify(ptBR)).not.toContain('""');
    expect(JSON.stringify(en)).not.toContain('""');
  });

  it("os namespaces pedidos pela spec existem", () => {
    expect(Object.keys(ptBR).sort()).toEqual(
      [
        "common",
        "dashboard",
        "devices",
        "errors",
        "latency",
        "mapping",
        "onboarding",
        "settings",
        "tray",
      ].sort(),
    );
  });
});

describe("pickLanguage", () => {
  it("usa o idioma salvo depois do onboarding", () => {
    expect(pickLanguage({ language: "en", onboarding_completed: true }, "pt-BR")).toBe("en");
    expect(pickLanguage({ language: "pt-BR", onboarding_completed: true }, "en-US")).toBe("pt-BR");
  });

  it("antes do onboarding segue o sistema: inglês vira EN, o resto PT-BR", () => {
    const fresh = { language: "pt-BR", onboarding_completed: false } as const;
    expect(pickLanguage(fresh, "en-US")).toBe("en");
    expect(pickLanguage(fresh, "en")).toBe("en");
    expect(pickLanguage(fresh, "pt-BR")).toBe("pt-BR");
    expect(pickLanguage(fresh, "de-DE")).toBe("pt-BR");
  });

  it("sem configurações (falha ao ler) também segue o sistema", () => {
    expect(pickLanguage(undefined, "en-GB")).toBe("en");
    expect(pickLanguage(undefined, "es")).toBe("pt-BR");
  });
});

describe("troca de idioma", () => {
  it("t() muda na hora e o <html lang> acompanha", async () => {
    expect(i18n.t("common:nav.settings")).toBe("Configurações");
    await setLanguage("en");
    expect(i18n.t("common:nav.settings")).toBe("Settings");
    expect(document.documentElement.lang).toBe("en");
  });

  it("as mensagens de erro do backend seguem o idioma", async () => {
    const error = { code: "no_network", message_key: "errors.no_network" } as const;
    expect(messageForError(error)).toContain("Nenhuma rede local");
    await setLanguage("en");
    expect(messageForError(error)).toContain("No local network");
  });

  it("interpola variáveis", () => {
    expect(i18n.t("common:comingSoon.description", { task: "T-603" })).toBe(
      "Esta tela chega na T-603.",
    );
  });
});

describe("formatação com Intl", () => {
  it("números e datas seguem o idioma", async () => {
    expect(formatNumber(1234.5)).toBe("1.234,5");
    await setLanguage("en");
    expect(formatNumber(1234.5)).toBe("1,234.5");
  });

  it("formata a data conforme o idioma", async () => {
    const date = new Date(2026, 9, 3, 14, 5);
    const pt = formatDateTime(date, { dateStyle: "short" });
    await setLanguage("en");
    const english = formatDateTime(date, { dateStyle: "short" });
    expect(pt).toBe("03/10/2026");
    expect(english).toBe("10/3/26");
  });
});
