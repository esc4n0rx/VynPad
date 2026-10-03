import { Smartphone } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ComingSoon } from "@/components/shell/ComingSoon";

export function DevicesPage() {
  const { t } = useTranslation("devices");
  return <ComingSoon icon={Smartphone} title={t("title")} task="T-603" />;
}
