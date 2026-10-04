import { useEffect, useRef } from "react";
import { useLocation, useNavigate } from "react-router";
import { useSettings } from "@/lib/api/settings";

export const ONBOARDING_PATH = "/onboarding";

/**
 * Na primeira abertura (introdução ainda não concluída), leva à introdução. Só na leitura inicial
 * das configurações: depois o usuário navega livre, e "Refazer introdução" abre a tela por conta
 * própria.
 */
export function useOnboardingRedirect(): void {
  const { data: settings } = useSettings();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const checked = useRef(false);

  useEffect(() => {
    if (checked.current || !settings) return;
    checked.current = true;
    if (!settings.onboarding_completed && pathname !== ONBOARDING_PATH) {
      void navigate(ONBOARDING_PATH, { replace: true });
    }
  }, [settings, pathname, navigate]);
}
