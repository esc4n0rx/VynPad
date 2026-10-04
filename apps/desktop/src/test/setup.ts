import "@/i18n";

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
