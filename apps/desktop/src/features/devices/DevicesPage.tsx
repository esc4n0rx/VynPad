import { useState } from "react";
import { useSearchParams } from "react-router";
import { Plus, Smartphone } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { useDevices } from "@/lib/api/devices";
import { useNearbyDevices } from "@/lib/api/server";
import { HistoryPanel } from "./HistoryPanel";
import { NearbySection } from "./NearbySection";
import { PairDialog } from "./PairDialog";
import { PairedSection } from "./PairedSection";

/** Celulares próximos, pareados e o histórico; daqui sai o pareamento. */
export function DevicesPage() {
  const { t } = useTranslation("devices");
  const devices = useDevices();
  const nearby = useNearbyDevices();
  const [pairing, setPairing] = useState<{ target?: string } | null>(null);
  // O menu da bandeja abre esta tela com `?pair=1`: já mostra o código de pareamento.
  const [params, setParams] = useSearchParams();
  const wantsPair = params.get("pair") === "1";

  const paired = devices.data ?? [];
  const around = nearby.data ?? [];
  const empty = paired.length === 0 && around.length === 0;

  return (
    <div className="flex min-h-full flex-col gap-6 p-6">
      <header className="flex items-center gap-3">
        <h1 className="flex-1 text-title font-semibold tracking-tight">{t("title")}</h1>
        <Button
          variant="primary"
          onClick={() => {
            setPairing({});
          }}
        >
          <Plus />
          {t("pairNew")}
        </Button>
      </header>

      {empty && devices.isSuccess && (
        <EmptyState
          icon={<Smartphone strokeWidth={1.5} />}
          title={t("empty.title")}
          description={t("empty.description")}
        />
      )}

      <NearbySection
        nearby={around}
        paired={paired}
        onPair={(device) => {
          setPairing({ target: device.name });
        }}
      />
      {paired.length > 0 && <PairedSection devices={paired} />}
      <HistoryPanel devices={paired} />

      <PairDialog
        open={pairing !== null || wantsPair}
        onOpenChange={(open) => {
          if (open) return;
          setPairing(null);
          if (wantsPair) setParams({}, { replace: true });
        }}
        target={pairing?.target}
      />
    </div>
  );
}
