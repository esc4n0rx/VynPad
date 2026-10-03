import { Outlet } from "react-router";
import { TitleBar } from "@/components/TitleBar";
import { useLanguageSync } from "@/i18n/useLanguageSync";

/** Moldura da janela: barra de título e, abaixo, a rota (com ou sem sidebar). */
export function RootLayout() {
  useLanguageSync();
  return (
    <div className="flex h-full flex-col">
      <TitleBar />
      <div className="min-h-0 flex-1">
        <Outlet />
      </div>
    </div>
  );
}
