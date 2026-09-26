import { Outlet } from "react-router";
import { TitleBar } from "@/components/TitleBar";

export function RootLayout() {
  return (
    <div className="flex h-full flex-col">
      <TitleBar />
      <main className="min-h-0 flex-1 overflow-auto">
        <Outlet />
      </main>
    </div>
  );
}
