import { useState } from "react";
import { Check } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Toggle } from "@/components/ui/toggle";
import type { Settings } from "@/lib/api/settings";
import { ACCENT_PALETTE } from "@/lib/theme";
import { cn } from "@/lib/utils";
import { SettingRow } from "./SettingRow";

interface SectionProps {
  settings: Settings;
  patch: (change: Partial<Settings>) => void;
}

const THEMES = ["dark", "light", "system"] as const;
const HEX = /^#[0-9a-f]{6}$/i;

function AccentPicker({ settings, patch }: SectionProps) {
  const { t } = useTranslation("settings");
  const current = settings.accent.toLowerCase();
  const preset = ACCENT_PALETTE.find((p) => p.color.toLowerCase() === current);
  // O seletor nativo dispara a cada movimento: só grava quando a cor for válida.
  const [custom, setCustom] = useState(current);

  return (
    <div
      role="radiogroup"
      aria-label={t("appearance.accent.group")}
      className="flex flex-wrap items-center gap-2"
    >
      {ACCENT_PALETTE.map(({ id, color }) => {
        const selected = preset?.id === id;
        return (
          <button
            key={id}
            type="button"
            role="radio"
            aria-checked={selected}
            aria-label={t(`appearance.accent.${id}`)}
            title={t(`appearance.accent.${id}`)}
            onClick={() => {
              patch({ accent: color });
            }}
            style={{ backgroundColor: color }}
            className={cn(
              "flex size-6 items-center justify-center rounded-full ring-offset-2 ring-offset-window focus-ring transition-transform hover:scale-110",
              selected && "ring-2 ring-fg",
            )}
          >
            {selected && <Check aria-hidden className="size-3.5 text-white" strokeWidth={3} />}
          </button>
        );
      })}
      <label
        title={t("appearance.accent.custom")}
        className={cn(
          "relative flex size-6 cursor-pointer items-center justify-center overflow-hidden rounded-full ring-offset-2 ring-offset-window focus-ring",
          !preset && "ring-2 ring-fg",
        )}
        style={{
          background: preset
            ? "conic-gradient(red, yellow, lime, aqua, blue, magenta, red)"
            : settings.accent,
        }}
      >
        <input
          type="color"
          aria-label={t("appearance.accent.customLabel")}
          value={HEX.test(custom) ? custom : current}
          onChange={(e) => {
            setCustom(e.target.value);
            if (HEX.test(e.target.value)) patch({ accent: e.target.value.toUpperCase() });
          }}
          className="absolute inset-0 size-full cursor-pointer opacity-0"
        />
      </label>
    </div>
  );
}

/** Aparência: tema, cor de destaque, transparência e movimento. */
export function AppearanceSection({ settings, patch }: SectionProps) {
  const { t } = useTranslation("settings");

  return (
    <>
      <SettingRow id="theme">
        {() => (
          <SegmentedControl
            label={t("items.theme.label")}
            value={settings.theme}
            options={THEMES.map((theme) => ({
              value: theme,
              label: t(`appearance.theme.${theme}`),
            }))}
            onValueChange={(theme) => {
              patch({ theme });
            }}
          />
        )}
      </SettingRow>
      <SettingRow id="accent">
        {() => <AccentPicker settings={settings} patch={patch} />}
      </SettingRow>
      <SettingRow id="transparency">
        {({ labelId, hintId }) => (
          <Toggle
            aria-labelledby={labelId}
            aria-describedby={hintId}
            checked={settings.transparency}
            onCheckedChange={(transparency) => {
              patch({ transparency });
            }}
          />
        )}
      </SettingRow>
      <SettingRow id="reduceMotion">
        {({ labelId, hintId }) => (
          <Toggle
            aria-labelledby={labelId}
            aria-describedby={hintId}
            checked={settings.reduce_motion}
            onCheckedChange={(reduce_motion) => {
              patch({ reduce_motion });
            }}
          />
        )}
      </SettingRow>
    </>
  );
}
