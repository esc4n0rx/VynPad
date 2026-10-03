import { Activity } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ComingSoon } from "@/components/shell/ComingSoon";

export function LatencyPage() {
  const { t } = useTranslation("latency");
  return <ComingSoon icon={Activity} title={t("title")} task="T-606" />;
}
