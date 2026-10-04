// Gestos comuns da interface nos testes E2E.
import { $, browser } from "@wdio/globals";

const quote = (text) => JSON.stringify(text);

/** XPath de qualquer elemento cujo texto próprio contém `text` (o `*=` do WebdriverIO só acha links). */
export const byText = (text) => `//*[text()[contains(normalize-space(.), ${quote(text)})]]`;

/** XPath de um botão pelo texto exato. */
export const button = (text) => `//button[normalize-space(.)=${quote(text)}]`;

/** Clica no primeiro botão (pelo texto exato) que aparecer, esperando até `timeout` ms. */
export async function pressAny(names, timeout = 20_000) {
  let pressed;
  await browser.waitUntil(
    async () => {
      for (const name of names) {
        const candidate = await $(button(name));
        if ((await candidate.isExisting()) && (await candidate.isEnabled())) {
          await candidate.click();
          pressed = name;
          return true;
        }
      }
      return false;
    },
    { timeout, timeoutMsg: `nenhum botão entre ${JSON.stringify(names)}` },
  );
  return pressed;
}

/** Vai a uma seção pela barra lateral. */
export async function goTo(section) {
  await $(
    `//*[@aria-label="Seções"]//a[contains(normalize-space(.), ${quote(section)})]`,
  ).click();
}

/**
 * Escolhe um item de um menu aberto (Radix) pelo teclado. Num submenu o clique do WebDriver
 * teletransporta o ponteiro e o Radix o fecha, então o caminho confiável é setas + Enter.
 */
export async function chooseMenuItem(label) {
  for (let i = 0; i < 12; i += 1) {
    await browser.keys(["ArrowDown"]);
    const focused = await browser.execute(() => document.activeElement?.textContent?.trim() ?? "");
    if (focused === label) {
      await browser.keys(["Enter"]);
      return;
    }
  }
  throw new Error(`o item de menu "${label}" não foi encontrado pelo teclado`);
}
