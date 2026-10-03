import { Outlet } from "react-router";
import { COLLAPSE_BELOW_PX } from "@/app/navigation";
import { PageTransition } from "@/components/shell/PageTransition";
import { Sidebar } from "@/components/shell/Sidebar";
import { useMediaQuery } from "@/lib/useMediaQuery";
import { useNavigationShortcuts } from "@/lib/useShortcuts";

/** Sidebar + conteúdo das seções. O onboarding fica fora dele (tela cheia). */
export function ShellLayout() {
  const collapsed = useMediaQuery(`(max-width: ${String(COLLAPSE_BELOW_PX - 1)}px)`);
  useNavigationShortcuts();

  return (
    <div className="flex h-full">
      <Sidebar collapsed={collapsed} />
      <main className="min-w-0 flex-1 overflow-auto">
        <PageTransition>
          <Outlet />
        </PageTransition>
      </main>
    </div>
  );
}
