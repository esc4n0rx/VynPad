import { Settings } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ComingSoon } from "@/components/shell/ComingSoon";

export function SettingsPage() {
  const { t } = useTranslation("settings");
  return <ComingSoon icon={Settings} title={t("title")} task="T-605" />;
}
