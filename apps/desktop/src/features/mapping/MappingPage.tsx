import { SlidersHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ComingSoon } from "@/components/shell/ComingSoon";

export function MappingPage() {
  const { t } = useTranslation("mapping");
  return <ComingSoon icon={SlidersHorizontal} title={t("title")} task="T-604" />;
}
