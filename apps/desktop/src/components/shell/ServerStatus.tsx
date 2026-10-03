import { useDriverStatus, useServerInfo } from "@/lib/api/server";
import { useSlots } from "@/lib/api/slots";
import { useTranslation } from "react-i18next";
import { driverLabelKey } from "@/components/shell/driverLabel";
import type { SlotStateDto } from "@/lib/bindings";
import { cn } from "@/lib/utils";

const SLOT_COLORS = ["bg-slot-1", "bg-slot-2", "bg-slot-3", "bg-slot-4"] as const;
const SLOT_RINGS = ["ring-slot-1", "ring-slot-2", "ring-slot-3", "ring-slot-4"] as const;

const STATE_KEY = {
  free: "slots.free",
  reserved: "slots.reserved",
  active: "slots.active",
  suspended: "slots.suspended",
} as const satisfies Record<SlotStateDto, string>;

/** Mini-indicadores P1–P4: cheio = conectado, anel = reservado/reconectando, apagado = livre. */
export function SlotDots({ className }: { className?: string }) {
  const { t } = useTranslation();
  const slots = useSlots((s) => s.slots);

  return (
    <ul aria-label={t("slots.label")} className={cn("flex items-center gap-1.5", className)}>
      {[0, 1, 2, 3].map((index) => {
        const state = slots?.slots.find((s) => s.slot === index)?.state ?? "free";
        const title = t("slots.title", { slot: index + 1, state: t(STATE_KEY[state]) });
        return (
          <li
            key={index}
            title={title}
            aria-label={title}
            className={cn(
              "size-2.5 rounded-full transition-colors",
              state === "active" && SLOT_COLORS[index],
              (state === "reserved" || state === "suspended") &&
                cn("ring-2 ring-inset", SLOT_RINGS[index]),
              state === "free" && "bg-control-hover",
            )}
          />
        );
      })}
    </ul>
  );
}

/** Estado do server no rodapé da sidebar: online, driver e IP da LAN. */
export function ServerStatus() {
  const { t } = useTranslation();
  const server = useServerInfo();
  const driver = useDriverStatus();
  const online = server.data !== undefined;
  const ip = server.data?.addresses[0]?.ip;

  return (
    <div className="flex flex-col gap-2 px-5 pb-4 text-caption">
      <div className="flex items-center gap-2 text-fg">
        <span
          aria-hidden
          className={cn("size-2 rounded-full", online ? "bg-success" : "bg-fg-3")}
        />
        <span className="font-medium">
          {online
            ? t("server.online")
            : server.isError
              ? t("server.offline")
              : t("server.starting")}
        </span>
      </div>
      <p className="text-fg-2">{t(driverLabelKey(driver.data))}</p>
      {ip && <p className="text-fg-3 tabular-nums">{ip}</p>}
      <SlotDots />
    </div>
  );
}
