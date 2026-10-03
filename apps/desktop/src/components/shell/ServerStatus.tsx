import { useDriverStatus, useServerInfo } from "@/lib/api/server";
import { useSlots } from "@/lib/api/slots";
import { driverLabel } from "@/components/shell/driverLabel";
import type { SlotStateDto } from "@/lib/bindings";
import { cn } from "@/lib/utils";

const SLOT_COLORS = ["bg-slot-1", "bg-slot-2", "bg-slot-3", "bg-slot-4"] as const;
const SLOT_RINGS = ["ring-slot-1", "ring-slot-2", "ring-slot-3", "ring-slot-4"] as const;

const STATE_LABEL: Record<SlotStateDto, string> = {
  free: "livre",
  reserved: "reservado",
  active: "conectado",
  suspended: "reconectando",
};

/** Mini-indicadores P1–P4: cheio = conectado, anel = reservado/reconectando, apagado = livre. */
export function SlotDots({ className }: { className?: string }) {
  const slots = useSlots((s) => s.slots);

  return (
    <ul aria-label="Controles" className={cn("flex items-center gap-1.5", className)}>
      {[0, 1, 2, 3].map((index) => {
        const state = slots?.slots.find((s) => s.slot === index)?.state ?? "free";
        return (
          <li
            key={index}
            title={`P${String(index + 1)}: ${STATE_LABEL[state]}`}
            aria-label={`P${String(index + 1)}: ${STATE_LABEL[state]}`}
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
          {online ? "Online" : server.isError ? "Offline" : "Iniciando…"}
        </span>
      </div>
      <p className="text-fg-2">{driverLabel(driver.data)}</p>
      {ip && <p className="text-fg-3 tabular-nums">{ip}</p>}
      <SlotDots />
    </div>
  );
}
