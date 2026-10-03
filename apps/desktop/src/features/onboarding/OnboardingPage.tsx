import { Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ComingSoon } from "@/components/shell/ComingSoon";

/** Tela cheia, sem sidebar. */
export function OnboardingPage() {
  const { t } = useTranslation("onboarding");
  return <ComingSoon icon={Sparkles} title={t("title")} task="T-601" />;
}
