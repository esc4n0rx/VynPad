import { useTranslation } from "react-i18next";
import { Select, type SelectOption } from "@/components/ui/select";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Slider } from "@/components/ui/slider";
import { Toggle } from "@/components/ui/toggle";
import { formatNumber } from "@/i18n/format";
import type { Button, GyroAxis, GyroMode, Profile } from "@/lib/bindings";
import { Field, ToggleRow } from "./Field";
import { ALL_BUTTONS, BUTTON_ELEMENT, GYRO_DEFAULTS, gyroOf, setGyro } from "./profileModel";

interface GyroPanelProps {
  profile: Profile;
  readOnly: boolean;
  edit: (change: (profile: Profile) => Profile) => void;
}

const AXES: readonly GyroAxis[] = ["yaw", "pitch", "roll"];
const decimal = (digits: number) => (value: number) =>
  formatNumber(value, { minimumFractionDigits: digits, maximumFractionDigits: digits });

/** O gyro do celular movendo um analógico (T-206). */
export function GyroPanel({ profile, readOnly, edit }: GyroPanelProps) {
  const { t } = useTranslation("mapping");
  const gyro = gyroOf(profile);
  const patch = (change: Parameters<typeof setGyro>[1]) => {
    edit((p) => setGyro(p, change));
  };

  const buttonOptions: SelectOption[] = ALL_BUTTONS.map((b: Button) => ({
    value: b,
    label: t(`elements.${BUTTON_ELEMENT[b]}`),
  }));
  const axisOptions: SelectOption[] = AXES.map((axis) => ({
    value: axis,
    label: t(`gyro.${axis}`),
  }));
  const modeButton: Button | null = gyro.mode.type === "always" ? null : gyro.mode.button;

  const setMode = (type: GyroMode["type"]) => {
    if (type === "always") patch({ mode: { type: "always" } });
    else patch({ mode: { type, button: modeButton ?? "left_shoulder" } });
  };

  return (
    <div className="flex flex-col gap-4">
      <h3 className="text-subtitle font-semibold text-fg">{t("gyro.title")}</h3>

      <ToggleRow label={t("gyro.enabled")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={gyro.enabled}
            disabled={readOnly}
            onCheckedChange={(enabled) => {
              patch({ enabled });
            }}
          />
        )}
      </ToggleRow>

      <Field label={t("gyro.mode")}>
        {() => (
          <SegmentedControl
            label={t("gyro.mode")}
            value={gyro.mode.type}
            options={[
              { value: "always", label: t("gyro.always") },
              { value: "while_held", label: t("gyro.whileHeld") },
              { value: "toggle", label: t("gyro.toggle") },
            ]}
            onValueChange={(type) => {
              if (!readOnly) setMode(type);
            }}
          />
        )}
      </Field>
      {modeButton !== null && (
        <Field label={t("gyro.button")}>
          {() => (
            <Select
              label={t("gyro.button")}
              options={buttonOptions}
              value={modeButton}
              disabled={readOnly}
              className="w-full"
              onValueChange={(button) => {
                patch({
                  mode: {
                    type: gyro.mode.type as "while_held" | "toggle",
                    button: button as Button,
                  },
                });
              }}
            />
          )}
        </Field>
      )}

      <Field label={t("gyro.target")}>
        {() => (
          <SegmentedControl
            label={t("gyro.target")}
            value={gyro.target}
            options={[
              { value: "left", label: t("gyro.left") },
              { value: "right", label: t("gyro.right") },
            ]}
            onValueChange={(target) => {
              if (!readOnly) patch({ target });
            }}
          />
        )}
      </Field>

      <Field label={t("gyro.axisX")}>
        {() => (
          <Select
            label={t("gyro.axisX")}
            options={axisOptions}
            value={gyro.axis_x}
            disabled={readOnly}
            className="w-full"
            onValueChange={(axis_x) => {
              patch({ axis_x: axis_x as GyroAxis });
            }}
          />
        )}
      </Field>
      <Field label={t("gyro.axisY")}>
        {() => (
          <Select
            label={t("gyro.axisY")}
            options={axisOptions}
            value={gyro.axis_y}
            disabled={readOnly}
            className="w-full"
            onValueChange={(axis_y) => {
              patch({ axis_y: axis_y as GyroAxis });
            }}
          />
        )}
      </Field>

      <Field label={t("gyro.sensitivityX")}>
        {() => (
          <Slider
            label={t("gyro.sensitivityX")}
            min={0}
            max={10}
            step={0.1}
            defaultValue={GYRO_DEFAULTS.sensitivity_x}
            value={gyro.sensitivity_x}
            disabled={readOnly}
            format={(v) => t("multiplier", { value: decimal(1)(v) })}
            onValueChange={(sensitivity_x) => {
              patch({ sensitivity_x });
            }}
          />
        )}
      </Field>
      <Field label={t("gyro.sensitivityY")}>
        {() => (
          <Slider
            label={t("gyro.sensitivityY")}
            min={0}
            max={10}
            step={0.1}
            defaultValue={GYRO_DEFAULTS.sensitivity_y}
            value={gyro.sensitivity_y}
            disabled={readOnly}
            format={(v) => t("multiplier", { value: decimal(1)(v) })}
            onValueChange={(sensitivity_y) => {
              patch({ sensitivity_y });
            }}
          />
        )}
      </Field>

      <ToggleRow label={t("gyro.invertX")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={gyro.invert_x}
            disabled={readOnly}
            onCheckedChange={(invert_x) => {
              patch({ invert_x });
            }}
          />
        )}
      </ToggleRow>
      <ToggleRow label={t("gyro.invertY")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={gyro.invert_y}
            disabled={readOnly}
            onCheckedChange={(invert_y) => {
              patch({ invert_y });
            }}
          />
        )}
      </ToggleRow>

      <Field label={t("gyro.deadzone")}>
        {() => (
          <Slider
            label={t("gyro.deadzone")}
            min={0}
            max={0.5}
            step={0.01}
            defaultValue={GYRO_DEFAULTS.deadzone}
            value={gyro.deadzone}
            disabled={readOnly}
            format={decimal(2)}
            onValueChange={(deadzone) => {
              patch({ deadzone });
            }}
          />
        )}
      </Field>

      <ToggleRow label={t("gyro.smoothing")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={gyro.smoothing.enabled}
            disabled={readOnly}
            onCheckedChange={(enabled) => {
              patch({ smoothing: { ...gyro.smoothing, enabled } });
            }}
          />
        )}
      </ToggleRow>
      {gyro.smoothing.enabled && (
        <>
          <Field label={t("gyro.cutoff")}>
            {() => (
              <Slider
                label={t("gyro.cutoff")}
                min={0.1}
                max={10}
                step={0.1}
                defaultValue={GYRO_DEFAULTS.smoothing.min_cutoff}
                value={gyro.smoothing.min_cutoff}
                disabled={readOnly}
                format={(v) => t("hz", { value: decimal(1)(v) })}
                onValueChange={(min_cutoff) => {
                  patch({ smoothing: { ...gyro.smoothing, min_cutoff } });
                }}
              />
            )}
          </Field>
          <Field label={t("gyro.beta")}>
            {() => (
              <Slider
                label={t("gyro.beta")}
                min={0}
                max={1}
                step={0.01}
                defaultValue={GYRO_DEFAULTS.smoothing.beta}
                value={gyro.smoothing.beta}
                disabled={readOnly}
                format={decimal(2)}
                onValueChange={(beta) => {
                  patch({ smoothing: { ...gyro.smoothing, beta } });
                }}
              />
            )}
          </Field>
        </>
      )}

      <Field label={t("gyro.recenter")}>
        {() => (
          <Select
            label={t("gyro.recenter")}
            options={[{ value: "none", label: t("gyro.recenterNone") }, ...buttonOptions]}
            value={gyro.recenter_button ?? "none"}
            disabled={readOnly}
            className="w-full"
            onValueChange={(value) => {
              patch({ recenter_button: value === "none" ? null : (value as Button) });
            }}
          />
        )}
      </Field>

      <ToggleRow label={t("gyro.mix")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={gyro.mix_with_stick}
            disabled={readOnly}
            onCheckedChange={(mix_with_stick) => {
              patch({ mix_with_stick });
            }}
          />
        )}
      </ToggleRow>
    </div>
  );
}
