import { useTranslation } from "react-i18next";
import { driverLabelKey } from "@/components/shell/driverLabel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Slider } from "@/components/ui/slider";
import { formatNumber } from "@/i18n/format";
import { useProfiles } from "@/lib/api/profiles";
import { useDriverStatus } from "@/lib/api/server";
import type { Settings } from "@/lib/api/settings";
import { useOpenVigemDownload } from "@/lib/api/system";
import { SettingRow } from "./SettingRow";
import { useDebouncedValue } from "./useDebouncedValue";

interface SectionProps {
  settings: Settings;
  patch: (change: Partial<Settings>) => void;
}

/** Valor do `Select` para "sem perfil escolhido" (o Radix não aceita `""` como item). */
const APP_DEFAULT = "app-default";

const SEND_RATES = ["125", "250"] as const;

function DriverRow() {
  const { t } = useTranslation();
  const { t: ts } = useTranslation("settings");
  const driver = useDriverStatus();
  const download = useOpenVigemDownload();
  const ready = driver.data?.state === "ready";

  return (
    <SettingRow id="driver">
      {() => (
        <div className="flex items-center gap-3">
          <Badge status={ready ? "success" : driver.data ? "warning" : "neutral"}>
            {t(driverLabelKey(driver.data))}
          </Badge>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              download.mutate();
            }}
          >
            {ready ? ts("controls.driver.reinstall") : ts("controls.driver.download")}
          </Button>
        </div>
      )}
    </SettingRow>
  );
}

/** Controles: driver, perfil padrão, taxa de envio e tempo de reserva. */
export function ControlsSection({ settings, patch }: SectionProps) {
  const { t } = useTranslation("settings");
  const profiles = useProfiles();
  const [reservation, setReservation] = useDebouncedValue(settings.reservation_secs, (secs) => {
    patch({ reservation_secs: secs });
  });

  return (
    <>
      <DriverRow />
      <SettingRow id="defaultProfile">
        {() => (
          <Select
            label={t("items.defaultProfile.label")}
            value={settings.default_profile_id ?? APP_DEFAULT}
            options={[
              { value: APP_DEFAULT, label: t("controls.defaultProfile.appDefault") },
              ...(profiles.data ?? [])
                .filter((p) => !p.is_builtin)
                .map((p) => ({ value: p.id, label: p.name })),
            ]}
            onValueChange={(value) => {
              patch({ default_profile_id: value === APP_DEFAULT ? null : value });
            }}
            className="w-56"
          />
        )}
      </SettingRow>
      <SettingRow id="sendRate">
        {() => (
          <SegmentedControl
            label={t("items.sendRate.label")}
            value={String(settings.send_rate_hz) as (typeof SEND_RATES)[number]}
            options={SEND_RATES.map((hz) => ({
              value: hz,
              label: t("controls.sendRate.hz", { hz }),
            }))}
            onValueChange={(hz) => {
              patch({ send_rate_hz: Number(hz) });
            }}
          />
        )}
      </SettingRow>
      <SettingRow id="reservation">
        {() => (
          <Slider
            label={t("items.reservation.label")}
            min={10}
            max={120}
            step={5}
            defaultValue={30}
            value={reservation}
            format={(v) => t("controls.reservation.seconds", { seconds: formatNumber(v) })}
            onValueChange={setReservation}
            className="w-64"
          />
        )}
      </SettingRow>
    </>
  );
}
