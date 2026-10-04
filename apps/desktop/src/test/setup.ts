import { configure } from "@testing-library/react";
import "@/i18n";

// Rotas e documentos da ajuda carregam sob demanda; num computador lento (o runner do CI) o
// prazo padrão de 1 s das esperas do Testing Library não basta. O teste termina assim que a
// condição vale, então o prazo maior só pesa quando algo está de fato errado.
configure({ asyncUtilTimeout: 5000 });

// O jsdom não implementa `ResizeObserver`, que o Radix (slider, popper) e o gráfico usam.
if (typeof globalThis.ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {
      /* sem layout no jsdom */
    }
    unobserve(): void {
      /* sem layout no jsdom */
    }
    disconnect(): void {
      /* sem layout no jsdom */
    }
  };
}
