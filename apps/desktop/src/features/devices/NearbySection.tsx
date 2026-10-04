import { useTranslation } from "react-i18next";
import { DeviceIcon } from "@/components/icons/DeviceIcon";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import type { DeviceDto, NearbyDto } from "@/lib/bindings";

/** Menos que isto conta como "visto agora" (o anúncio chega a cada poucos segundos). */
const NOW_MS = 3000;

interface NearbySectionProps {
  nearby: readonly NearbyDto[];
  paired: readonly DeviceDto[];
  onPair: (device: NearbyDto) => void;
}

/** Celulares anunciando na rede (T-303): os novos têm o botão de parear. */
export function NearbySection({ nearby, paired, onPair }: NearbySectionProps) {
  const { t } = useTranslation("devices");

  return (
    <section aria-labelledby="nearby-title" className="flex flex-col gap-3">
      <h2 id="nearby-title" className="text-subtitle font-semibold text-fg">
        {t("nearby.title")}
      </h2>
      {nearby.length === 0 ? (
        <p className="text-body text-fg-2">{t("nearby.empty")}</p>
      ) : (
        <ul className="grid grid-cols-1 gap-3 min-[860px]:grid-cols-2">
          {nearby.map((device) => {
            const known = paired.find((p) => p.id === device.id);
            const state = !device.paired
              ? "new"
              : known?.current_slot !== null && known?.current_slot !== undefined
                ? "connected"
                : "paired";
            return (
              <li key={device.id}>
                <Card className="flex items-center gap-3">
                  <DeviceIcon status={state === "connected" ? "connected" : "idle"} decorative />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-body font-medium text-fg">{device.name}</p>
                    <p className="truncate text-caption text-fg-3">
                      {device.model} · <span className="tabular-nums">{device.ip}</span>
                    </p>
                    <p className="text-caption text-fg-3">
                      {device.seen_ago_ms < NOW_MS
                        ? t("nearby.seenNow")
                        : t("nearby.seenAgo", { seconds: Math.round(device.seen_ago_ms / 1000) })}
                    </p>
                  </div>
                  <Badge
                    status={
                      state === "new" ? "info" : state === "connected" ? "success" : "neutral"
                    }
                  >
                    {t(`nearby.state.${state}`)}
                  </Badge>
                  {state === "new" && (
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={() => {
                        onPair(device);
                      }}
                    >
                      {t("nearby.pair")}
                    </Button>
                  )}
                </Card>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
