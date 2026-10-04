import { Outlet } from "react-router";
import { TitleBar } from "@/components/TitleBar";
import { useLanguageSync } from "@/i18n/useLanguageSync";
import { useReportVisibility } from "@/lib/api/metrics";
import { useAppearanceSync } from "@/lib/appearance";
import { useOnboardingRedirect } from "@/lib/useOnboardingRedirect";
import { useTrayNavigation } from "@/lib/useTrayNavigation";
import { ApprovalDialog } from "@/features/devices/ApprovalDialog";

/** Moldura da janela: barra de título e, abaixo, a rota (com ou sem sidebar). */
export function RootLayout() {
  useLanguageSync();
  useReportVisibility();
  useAppearanceSync();
  useTrayNavigation();
  useOnboardingRedirect();
  return (
    <div className="flex h-full flex-col">
      <TitleBar />
      <div className="min-h-0 flex-1">
        <Outlet />
      </div>
      <ApprovalDialog />
    </div>
  );
}
