import { useTranslation } from "react-i18next";
import type { ButtonElement } from "@/components/gamepad/elements";
import { Select, type SelectOption } from "@/components/ui/select";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Slider } from "@/components/ui/slider";
import { Toggle } from "@/components/ui/toggle";
import { formatNumber } from "@/i18n/format";
import type { Button, Profile, StickSide } from "@/lib/bindings";
import { CurveEditor } from "./CurveEditor";
import { DeadzoneDiagram } from "./DeadzoneDiagram";
import { Field, ToggleRow } from "./Field";
import {
  ALL_BUTTONS,
  BUTTON_ELEMENT,
  ELEMENT_BUTTON,
  STICK_DEFAULTS,
  TRIGGER_DEFAULTS,
  TURBO_DEFAULT_HZ,
  TURBO_MAX_HZ,
  TURBO_MIN_HZ,
  curveOfType,
  remapOf,
  setRemap,
  setStick,
  setSwapSticks,
  setTrigger,
  setTurbo,
  stickOf,
  triggerOf,
  turboOf,
} from "./profileModel";

interface PanelProps {
  profile: Profile;
  readOnly: boolean;
  edit: (change: (profile: Profile) => Profile) => void;
}

const pct = (value: number) => formatNumber(Math.round(value * 100));
const decimal = (digits: number) => (value: number) =>
  formatNumber(value, { minimumFractionDigits: digits, maximumFractionDigits: digits });

// ------------------------------------------------------------------ botão

export function ButtonPanel({
  element,
  profile,
  readOnly,
  edit,
}: PanelProps & { element: ButtonElement }) {
  const { t } = useTranslation("mapping");
  const button = ELEMENT_BUTTON[element];
  const target = remapOf(profile, button) ?? button;
  const turbo = turboOf(profile, button);

  const options: SelectOption[] = [
    { value: "none", label: t("button.none") },
    { value: "left_trigger", label: t("button.leftTrigger") },
    { value: "right_trigger", label: t("button.rightTrigger") },
    ...ALL_BUTTONS.map((b: Button) => ({ value: b, label: t(`elements.${BUTTON_ELEMENT[b]}`) })),
  ];

  return (
    <div className="flex flex-col gap-4">
      <h3 className="text-subtitle font-semibold text-fg">
        {t("button.title", { name: t(`elements.${element}`) })}
      </h3>
      <Field label={t("button.mapTo")}>
        {() => (
          <Select
            label={t("button.mapTo")}
            options={options}
            value={target}
            disabled={readOnly}
            className="w-full"
            onValueChange={(value) => {
              edit((p) => setRemap(p, button, value));
            }}
          />
        )}
      </Field>
      <ToggleRow label={t("button.turbo")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={turbo !== undefined}
            disabled={readOnly}
            onCheckedChange={(on) => {
              edit((p) => setTurbo(p, button, on ? TURBO_DEFAULT_HZ : null));
            }}
          />
        )}
      </ToggleRow>
      {turbo && (
        <Field label={t("button.turboRate")}>
          {() => (
            <Slider
              label={t("button.turboRate")}
              min={TURBO_MIN_HZ}
              max={TURBO_MAX_HZ}
              step={1}
              defaultValue={TURBO_DEFAULT_HZ}
              value={turbo.hz ?? TURBO_DEFAULT_HZ}
              disabled={readOnly}
              format={(v) => t("hz", { value: formatNumber(v) })}
              onValueChange={(hz) => {
                edit((p) => setTurbo(p, button, hz));
              }}
            />
          )}
        </Field>
      )}
    </div>
  );
}

// ------------------------------------------------------------------ analógico

export function StickPanel({ side, profile, readOnly, edit }: PanelProps & { side: StickSide }) {
  const { t } = useTranslation("mapping");
  const stick = stickOf(profile, side);
  const patch = (change: Parameters<typeof setStick>[2]) => {
    edit((p) => setStick(p, side, change));
  };

  return (
    <div className="flex flex-col gap-4">
      <h3 className="text-subtitle font-semibold text-fg">
        {t("stick.title", { name: t(`elements.stick-${side}`) })}
      </h3>

      <div className="flex flex-wrap items-center gap-4">
        <DeadzoneDiagram
          inner={stick.deadzone_inner}
          outer={stick.deadzone_outer}
          shape={stick.deadzone_shape}
          label={t("stick.diagram", {
            inner: pct(stick.deadzone_inner),
            outer: pct(stick.deadzone_outer),
          })}
          onChange={(change) => {
            if (!readOnly) patch(change);
          }}
        />
        <div className="flex min-w-48 flex-1 flex-col gap-3">
          <Field label={t("stick.deadzoneInner")}>
            {() => (
              <Slider
                label={t("stick.deadzoneInner")}
                min={0}
                max={0.5}
                step={0.01}
                defaultValue={STICK_DEFAULTS.deadzone_inner}
                value={stick.deadzone_inner}
                disabled={readOnly}
                format={(v) => t("percent", { value: pct(v) })}
                onValueChange={(v) => {
                  patch({ deadzone_inner: v });
                }}
              />
            )}
          </Field>
          <Field label={t("stick.deadzoneOuter")}>
            {() => (
              <Slider
                label={t("stick.deadzoneOuter")}
                min={0}
                max={0.5}
                step={0.01}
                defaultValue={STICK_DEFAULTS.deadzone_outer}
                value={stick.deadzone_outer}
                disabled={readOnly}
                format={(v) => t("percent", { value: pct(v) })}
                onValueChange={(v) => {
                  patch({ deadzone_outer: v });
                }}
              />
            )}
          </Field>
        </div>
      </div>

      <Field label={t("stick.shape")}>
        {() => (
          <SegmentedControl
            label={t("stick.shape")}
            value={stick.deadzone_shape}
            options={[
              { value: "radial", label: t("stick.radial") },
              { value: "axial", label: t("stick.axial") },
            ]}
            onValueChange={(deadzone_shape) => {
              if (!readOnly) patch({ deadzone_shape });
            }}
          />
        )}
      </Field>

      <Field label={t("stick.anti")}>
        {() => (
          <Slider
            label={t("stick.anti")}
            min={0}
            max={0.5}
            step={0.01}
            defaultValue={STICK_DEFAULTS.anti_deadzone}
            value={stick.anti_deadzone}
            disabled={readOnly}
            format={(v) => t("percent", { value: pct(v) })}
            onValueChange={(v) => {
              patch({ anti_deadzone: v });
            }}
          />
        )}
      </Field>

      <Field label={t("stick.sensitivity")}>
        {() => (
          <Slider
            label={t("stick.sensitivity")}
            min={0.1}
            max={4}
            step={0.05}
            defaultValue={STICK_DEFAULTS.sensitivity}
            value={stick.sensitivity}
            disabled={readOnly}
            format={(v) => t("multiplier", { value: decimal(2)(v) })}
            onValueChange={(v) => {
              patch({ sensitivity: v });
            }}
          />
        )}
      </Field>

      <Field label={t("curve.type")}>
        {() => (
          <div className="flex flex-wrap items-start gap-4">
            <CurveEditor
              curve={stick.curve}
              onChange={(curve) => {
                if (!readOnly) patch({ curve });
              }}
            />
            <div className="flex min-w-40 flex-1 flex-col gap-3">
              <SegmentedControl
                label={t("curve.type")}
                value={stick.curve.type}
                options={[
                  { value: "linear", label: t("curve.linear") },
                  { value: "exponential", label: t("curve.exponential") },
                  { value: "custom", label: t("curve.custom") },
                ]}
                onValueChange={(type) => {
                  if (!readOnly) patch({ curve: curveOfType(type, stick.curve) });
                }}
              />
              {stick.curve.type === "exponential" && (
                <Slider
                  label={t("curve.exponent")}
                  min={0.25}
                  max={4}
                  step={0.05}
                  defaultValue={2}
                  value={stick.curve.exponent ?? 2}
                  disabled={readOnly}
                  format={decimal(2)}
                  onValueChange={(exponent) => {
                    patch({ curve: { type: "exponential", exponent } });
                  }}
                />
              )}
            </div>
          </div>
        )}
      </Field>

      <ToggleRow label={t("stick.invertX")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={stick.invert_x}
            disabled={readOnly}
            onCheckedChange={(invert_x) => {
              patch({ invert_x });
            }}
          />
        )}
      </ToggleRow>
      <ToggleRow label={t("stick.invertY")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={stick.invert_y}
            disabled={readOnly}
            onCheckedChange={(invert_y) => {
              patch({ invert_y });
            }}
          />
        )}
      </ToggleRow>
      <ToggleRow label={t("stick.swap")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={profile.sticks?.swap ?? false}
            disabled={readOnly}
            onCheckedChange={(swap) => {
              edit((p) => setSwapSticks(p, swap));
            }}
          />
        )}
      </ToggleRow>
    </div>
  );
}

// ------------------------------------------------------------------ gatilho

export function TriggerPanel({ side, profile, readOnly, edit }: PanelProps & { side: StickSide }) {
  const { t } = useTranslation("mapping");
  const trigger = triggerOf(profile, side);
  const patch = (change: Parameters<typeof setTrigger>[2]) => {
    edit((p) => setTrigger(p, side, change));
  };

  return (
    <div className="flex flex-col gap-4">
      <h3 className="text-subtitle font-semibold text-fg">
        {t("trigger.title", { name: t(side === "left" ? "elements.lt" : "elements.rt") })}
      </h3>
      <Field label={t("trigger.deadzone")}>
        {() => (
          <Slider
            label={t("trigger.deadzone")}
            min={0}
            max={0.9}
            step={0.01}
            defaultValue={TRIGGER_DEFAULTS.deadzone}
            value={trigger.deadzone}
            disabled={readOnly}
            format={(v) => t("percent", { value: pct(v) })}
            onValueChange={(deadzone) => {
              patch({ deadzone });
            }}
          />
        )}
      </Field>
      <Field label={t("trigger.fullPoint")}>
        {() => (
          <Slider
            label={t("trigger.fullPoint")}
            min={0.1}
            max={1}
            step={0.01}
            defaultValue={TRIGGER_DEFAULTS.full_point}
            value={trigger.full_point}
            disabled={readOnly}
            format={(v) => t("percent", { value: pct(v) })}
            onValueChange={(full_point) => {
              patch({ full_point });
            }}
          />
        )}
      </Field>
      <ToggleRow label={t("trigger.digital")}>
        {(labelId) => (
          <Toggle
            aria-labelledby={labelId}
            checked={trigger.digital}
            disabled={readOnly}
            onCheckedChange={(digital) => {
              patch({ digital });
            }}
          />
        )}
      </ToggleRow>
    </div>
  );
}
