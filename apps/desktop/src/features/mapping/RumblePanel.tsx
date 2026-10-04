import { useTranslation } from "react-i18next";
import { Slider } from "@/components/ui/slider";
import { Toggle } from "@/components/ui/toggle";
import { formatNumber } from "@/i18n/format";
import type { Profile } from "@/lib/bindings";
import { Field, ToggleRow } from "./Field";
import { RUMBLE_DEFAULTS, RUMBLE_MAX, rumbleOf, setRumble } from "./profileModel";

interface RumblePanelProps {
  profile: Profile;
  readOnly: boolean;
  edit: (change: (profile: Profile) => Profile) => void;
}

/** Vibração: liga e desliga o que o jogo pede e escolhe a intensidade (0–200%). */
export function RumblePanel({ profile, readOnly, edit }: RumblePanelProps) {
  const { t } = useTranslation("mapping");
  const rumble = rumbleOf(profile);

  return (
    <div className="flex flex-col gap-4">
      <h3 className="text-subtitle font-semibold text-fg">{t("rumble.title")}</h3>
      <p className="text-caption text-fg-2">{t("rumble.hint")}</p>
      <ToggleRow label={t("rumble.enabled")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={rumble.enabled}
            disabled={readOnly}
            onCheckedChange={(enabled) => {
              edit((p) => setRumble(p, { enabled }));
            }}
          />
        )}
      </ToggleRow>
      <Field label={t("rumble.intensity")}>
        {() => (
          <Slider
            label={t("rumble.intensity")}
            min={0}
            max={RUMBLE_MAX}
            step={0.05}
            defaultValue={RUMBLE_DEFAULTS.intensity}
            value={rumble.intensity}
            disabled={readOnly || !rumble.enabled}
            format={(v) => t("rumble.percent", { value: formatNumber(Math.round(v * 100)) })}
            onValueChange={(intensity) => {
              edit((p) => setRumble(p, { intensity }));
            }}
          />
        )}
      </Field>
    </div>
  );
}
