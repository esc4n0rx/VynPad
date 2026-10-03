import { useTranslation } from "react-i18next";
import { i18n } from "./index";

/** Número no formato do idioma atual (`1.234,5` em PT-BR, `1,234.5` em EN). */
export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(i18n.language, options).format(value);
}

/** Data/hora no formato do idioma atual; aceita `Date` ou epoch em ms. */
export function formatDateTime(
  value: Date | number,
  options: Intl.DateTimeFormatOptions = { dateStyle: "short", timeStyle: "short" },
): string {
  return new Intl.DateTimeFormat(i18n.language, options).format(value);
}

/**
 * As mesmas funções, mas o componente re-renderiza quando o idioma muda
 * (`useTranslation` assina a troca).
 */
export function useFormatters() {
  useTranslation();
  return { formatNumber, formatDateTime };
}
