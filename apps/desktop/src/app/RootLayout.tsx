import { Outlet } from "react-router";
import { TitleBar } from "@/components/TitleBar";

/** Moldura da janela: barra de título e, abaixo, a rota (com ou sem sidebar). */
export function RootLayout() {
  return (
    <div className="flex h-full flex-col">
      <TitleBar />
      <div className="min-h-0 flex-1">
        <Outlet />
      </div>
    </div>
  );
}
