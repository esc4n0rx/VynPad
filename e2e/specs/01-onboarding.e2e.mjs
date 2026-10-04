import { $, expect } from "@wdio/globals";
import { byText, pressAny } from "../lib/ui.mjs";

describe("introdução", () => {
  it("abre na introdução quando a pasta de dados é nova", async () => {
    await expect($(byText("Seu celular é o controle"))).toBeDisplayed();
  });

  it("percorre os cinco passos e chega ao dashboard", async () => {
    // Boas-vindas.
    await pressAny(["Continuar"]);

    // Driver: com o ViGEmBus instalado segue com "Continuar"; sem ele, "Continuar sem driver".
    await expect($(byText("Driver de controle virtual"))).toBeDisplayed();
    await pressAny(["Continuar", "Continuar sem driver"]);

    // Rede.
    await expect($(byText("Rede"))).toBeDisplayed();
    await pressAny(["Continuar"]);

    // Celular: pula, o pareamento é o próximo teste.
    await expect($(byText("Conecte o celular"))).toBeDisplayed();
    await pressAny(["Pular por enquanto", "Continuar"]);

    // Pronto.
    await expect($(byText("Tudo pronto"))).toBeDisplayed();
    await pressAny(["Começar"]);

    await expect($(byText("Aguardando conexão"))).toBeDisplayed();
  });
});
