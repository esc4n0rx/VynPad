import { Gamepad2 } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { useAppInfo } from "@/lib/api/app";

// Tela provisória que valida a ponte React ↔ Rust. Substituída pelo dashboard (T-602).
export function HomePage() {
  const { t } = useTranslation("dashboard");
  const { data, error } = useAppInfo();

  return (
    <div className="flex h-full flex-col items-center justify-center gap-4">
      <motion.div
        initial={{ opacity: 0, scale: 0.9 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ type: "spring", stiffness: 380, damping: 32 }}
        className="flex size-20 items-center justify-center rounded-3xl bg-gradient-to-br from-accent to-indigo shadow-lg"
      >
        <Gamepad2 className="size-10 text-on-accent" strokeWidth={1.75} />
      </motion.div>
      <h1 className="text-[28px] font-semibold tracking-tight">PadGaming</h1>
      <p className="text-fg-2 tabular-nums">
        {error
          ? t("backendError")
          : data
            ? t("version", { app: data.app_version, core: data.core_version })
            : t("loading")}
      </p>
    </div>
  );
}
