import { createContext } from "react";
import type { ItemId } from "./settingsIndex";

/** Quais configurações a busca deixa à vista (`null` = todas, sem filtro). */
export const VisibleItems = createContext<ReadonlySet<ItemId> | null>(null);
