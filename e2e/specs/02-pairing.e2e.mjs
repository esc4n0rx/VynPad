import { $, browser, expect } from "@wdio/globals";
import { pairByPin } from "../lib/sim.mjs";
import { byText, goTo, pressAny } from "../lib/ui.mjs";

describe("pareamento", () => {
  // Um diálogo que sobrou de um teste que falhou não pode bloquear os seguintes.
  beforeEach(async () => {
    await browser.keys(["Escape"]);
  });

  it("pareia um celular pelo PIN mostrado na tela", async () => {
    await goTo("Dispositivos");
    await expect($(byText("Nenhum dispositivo ainda"))).toBeDisplayed();

    await pressAny(["Parear novo"]);
    await expect($(byText("Parear novo dispositivo"))).toBeDisplayed();
    await $('//*[@role="radio"][normalize-space(.)="PIN"]').click();

    // O PIN aparece em seis dígitos (talvez separados por espaço).
    const pinBox = await $('[aria-label="PIN de pareamento"]');
    await pinBox.waitForDisplayed();
    const pin = (await pinBox.getText()).replace(/\D/g, "");
    expect(pin).toHaveLength(6);

    // O simulador digita o PIN e fica conectado por um tempo (ocupa o slot P1).
    const phone = pairByPin({ pin, name: "Celular E2E", holdSeconds: 25 });
    phone.done.catch(() => {});
    try {
      await expect($(byText("Celular E2E pareado!"))).toBeDisplayed({ wait: 20_000 });
      await pressAny(["Concluir"]);

      // Entra na lista de pareados e aparece conectado no dashboard.
      await expect($(byText("Celular E2E"))).toBeDisplayed();
      await goTo("Controles");
      await expect($(byText("Celular E2E"))).toBeDisplayed();
    } finally {
      phone.stop();
    }
  });

  it("recusa um PIN errado sem parear", async () => {
    await goTo("Dispositivos");
    const phone = pairByPin({ pin: "000000", name: "Intruso E2E" });
    await expect(phone.done).rejects.toThrow(/recusou/);
    await browser.pause(500);
    await expect($(byText("Intruso E2E"))).not.toBeExisting();
  });
});
