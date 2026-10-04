import { useState } from "react";
import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import { formatDateTime } from "@/i18n/format";
import { useConnectionLog } from "@/lib/api/devices";
import type { DeviceDto } from "@/lib/bindings";
import { cn } from "@/lib/utils";

/** Quantos eventos o painel mostra. */
export const HISTORY_LIMIT = 50;

const KNOWN = ["connected", "disconnected", "rejected", "paired", "revoked"] as const;
type KnownEvent = (typeof KNOWN)[number];
const isKnown = (event: string): event is KnownEvent =>
  (KNOWN as readonly string[]).includes(event);

const QUERY = { device_id: null, before_id: null, limit: HISTORY_LIMIT } as const;

/** Últimos eventos de conexão. Recolhido por padrão: só consulta o banco quando abre. */
export function HistoryPanel({ devices }: { devices: readonly DeviceDto[] }) {
  const { t } = useTranslation("devices");
  const [open, setOpen] = useState(false);
  const log = useConnectionLog(QUERY, open);

  return (
    <section aria-labelledby="history-title" className="flex flex-col gap-3">
      <h2 id="history-title" className="text-subtitle font-semibold text-fg">
        <button
          type="button"
          aria-expanded={open}
          aria-controls="history-body"
          onClick={() => {
            setOpen((v) => !v);
          }}
          className="flex items-center gap-1.5 rounded-sm focus-ring"
        >
          <ChevronDown
            aria-hidden
            className={cn("size-4 text-fg-2 transition-transform", !open && "-rotate-90")}
          />
          {t("history.title")}
        </button>
      </h2>
      {open && (
        <div id="history-body">
          {log.data?.length === 0 ? (
            <p className="text-body text-fg-2">{t("history.empty")}</p>
          ) : (
            <ul className="flex flex-col divide-y divide-separator rounded-lg glass">
              {log.data?.map((entry) => {
                const device = devices.find((d) => d.id === entry.device_id);
                return (
                  <li key={entry.id} className="flex items-baseline gap-3 px-4 py-2 text-body">
                    <span className="w-36 shrink-0 text-caption text-fg-3 tabular-nums">
                      {formatDateTime(entry.at)}
                    </span>
                    <span className="font-medium text-fg">
                      {t(`history.events.${isKnown(entry.event) ? entry.event : "unknown"}`)}
                    </span>
                    <span className="min-w-0 flex-1 truncate text-fg-2">
                      {entry.device_id === null ? "" : (device?.name ?? t("history.unknownDevice"))}
                      {entry.slot !== null && ` · ${t("history.slot", { slot: entry.slot + 1 })}`}
                      {entry.detail && ` · ${entry.detail}`}
                    </span>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
