import { useEffect } from "react";
import { useSettings } from "@/lib/api/settings";
import { pickLanguage, setLanguage } from "./index";

/**
 * Acompanha o idioma das configurações: trocar na tela de Configurações (ou em outra janela,
 * pelo evento `settingsChanged`) atualiza a UI na hora.
 */
export function useLanguageSync(): void {
  const { data: settings } = useSettings();

  useEffect(() => {
    if (settings) void setLanguage(pickLanguage(settings));
  }, [settings]);
}
