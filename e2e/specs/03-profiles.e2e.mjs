import { $, expect } from "@wdio/globals";
import { byText, chooseMenuItem, goTo } from "../lib/ui.mjs";

describe("perfis", () => {
  it("duplica o perfil Padrão e troca o perfil de um dispositivo pareado", async () => {
    // Um perfil novo a partir do Padrão.
    await goTo("Mapeamento");
    await $('[aria-label="Ações do perfil Padrão"]').click();
    await $(byText("Duplicar")).click();
    await expect($(byText("Padrão (cópia)"))).toBeDisplayed();

    // O dispositivo pareado no teste anterior recebe o perfil novo.
    await goTo("Dispositivos");
    await $('[aria-label="Ações de Celular E2E"]').click();
    await $(byText("Definir perfil")).click();
    await chooseMenuItem("Padrão (cópia)");
    await expect($(byText("Perfil: Padrão (cópia)"))).toBeDisplayed();
  });
});
