import type { ParseKeys } from "i18next";

export type SectionId =
  | "general"
  | "network"
  | "controls"
  | "appearance"
  | "notifications"
  | "language"
  | "privacy"
  | "about";

export const SECTIONS: readonly SectionId[] = [
  "general",
  "network",
  "controls",
  "appearance",
  "notifications",
  "language",
  "privacy",
  "about",
];

export type ItemId =
  | "serverName"
  | "autostart"
  | "startMinimized"
  | "closeToTray"
  | "requireApproval"
  | "redoIntro"
  | "ports"
  | "interfaces"
  | "testConnection"
  | "firewall"
  | "driver"
  | "defaultProfile"
  | "sendRate"
  | "reservation"
  | "theme"
  | "accent"
  | "transparency"
  | "reduceMotion"
  | "notifyConnections"
  | "notifyPairing"
  | "notifyDriver"
  | "notifySkipFullscreen"
  | "language"
  | "clearHistory"
  | "regenerateIdentity"
  | "version"
  | "logs"
  | "help"
  | "updates"
  | "diagnostic"
  | "licenses";

export interface IndexedItem {
  id: ItemId;
  section: SectionId;
}

/** Todas as configurações, na ordem em que aparecem. A busca e a navegação partem desta lista. */
export const ITEMS: readonly IndexedItem[] = [
  { id: "serverName", section: "general" },
  { id: "autostart", section: "general" },
  { id: "startMinimized", section: "general" },
  { id: "closeToTray", section: "general" },
  { id: "requireApproval", section: "general" },
  { id: "redoIntro", section: "general" },
  { id: "ports", section: "network" },
  { id: "interfaces", section: "network" },
  { id: "testConnection", section: "network" },
  { id: "firewall", section: "network" },
  { id: "driver", section: "controls" },
  { id: "defaultProfile", section: "controls" },
  { id: "sendRate", section: "controls" },
  { id: "reservation", section: "controls" },
  { id: "theme", section: "appearance" },
  { id: "accent", section: "appearance" },
  { id: "transparency", section: "appearance" },
  { id: "reduceMotion", section: "appearance" },
  { id: "notifyConnections", section: "notifications" },
  { id: "notifyPairing", section: "notifications" },
  { id: "notifyDriver", section: "notifications" },
  { id: "notifySkipFullscreen", section: "notifications" },
  { id: "language", section: "language" },
  { id: "clearHistory", section: "privacy" },
  { id: "regenerateIdentity", section: "privacy" },
  { id: "version", section: "about" },
  { id: "logs", section: "about" },
  { id: "help", section: "about" },
  { id: "updates", section: "about" },
  { id: "diagnostic", section: "about" },
  { id: "licenses", section: "about" },
];

export const itemLabelKey = (id: ItemId): ParseKeys<"settings"> => `items.${id}.label`;
export const itemHintKey = (id: ItemId): ParseKeys<"settings"> => `items.${id}.hint`;

/** Tira acentos e caixa: "Conexão" casa com "conexao". */
export function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .trim();
}

/**
 * Quais configurações aparecem para a busca `query`. `text` devolve o texto pesquisável de uma
 * configuração (rótulo e dica, no idioma atual). Busca vazia mostra todas.
 */
export function matchingItems(query: string, text: (id: ItemId) => string): ReadonlySet<ItemId> {
  const needle = normalize(query);
  const all = ITEMS.map((i) => i.id);
  if (needle === "") return new Set(all);
  const words = needle.split(/\s+/);
  return new Set(
    all.filter((id) => {
      const haystack = normalize(text(id));
      return words.every((w) => haystack.includes(w));
    }),
  );
}
