import { useEffect, useMemo, useState } from "react";
import { Undo2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Gamepad } from "@/components/gamepad/Gamepad";
import {
  BUTTON_ELEMENTS,
  slotColor,
  type ButtonElement,
  type GamepadElement,
} from "@/components/gamepad/elements";
import { Badge } from "@/components/ui/badge";
import { IconButton } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Select, type SelectOption } from "@/components/ui/select";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Skeleton } from "@/components/ui/skeleton";
import { useProfile, useProfiles } from "@/lib/api/profiles";
import { useSlots } from "@/lib/api/slots";
import type { ProfileDto } from "@/lib/bindings";
import { ButtonPanel, StickPanel, TriggerPanel } from "./ControlPanels";
import { GyroPanel } from "./GyroPanel";
import { RumblePanel } from "./RumblePanel";
import { ProfileIcon } from "./ProfileIcon";
import { PROFILE_ICON_NAMES } from "./profileIcons";
import { ProfileList } from "./ProfileList";
import { useProfileEditor, type SaveState } from "./useProfileEditor";

type Tab = "controls" | "gyro" | "rumble";

const isButton = (element: GamepadElement): element is ButtonElement =>
  (BUTTON_ELEMENTS as readonly string[]).includes(element);

const SAVE_LABEL: Record<SaveState, string | null> = {
  idle: null,
  pending: "save.pending",
  saving: "save.saving",
  saved: "save.saved",
  error: "save.error",
};

function ProfileLoader({ id }: { id: string }) {
  const profile = useProfile(id);
  if (!profile.data) {
    return (
      <>
        <Skeleton className="h-full rounded-lg" />
        <Skeleton className="h-full rounded-lg" />
      </>
    );
  }
  return <Editor dto={profile.data} />;
}

/** Editor de mapeamento: perfis | controle interativo | propriedades. */
export function MappingPage() {
  const profiles = useProfiles();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const list = profiles.data;
  // Sem escolha ainda (ou o perfil escolhido foi excluído): o primeiro da lista.
  const activeId =
    selectedId !== null && list?.some((p) => p.id === selectedId)
      ? selectedId
      : (list?.[0]?.id ?? null);

  return (
    <div className="grid h-full min-h-0 grid-cols-[13rem_minmax(0,1fr)_21rem] gap-4 p-4">
      <ProfileList profiles={list ?? []} selectedId={activeId} onSelect={setSelectedId} />
      {activeId === null ? (
        <div className="col-span-2" />
      ) : (
        <ProfileLoader key={activeId} id={activeId} />
      )}
    </div>
  );
}

function Editor({ dto }: { dto: ProfileDto }) {
  const { t } = useTranslation("mapping");
  const editor = useProfileEditor(dto);
  const { draft, readOnly, saveState, canUndo } = editor;
  const [element, setElement] = useState<GamepadElement | null>(null);
  const [tab, setTab] = useState<Tab>("controls");
  const slots = useSlots((s) => s.slots);
  const connected = useMemo(
    () => slots?.slots.filter((s) => s.state !== "free").map((s) => s.slot) ?? [],
    [slots],
  );
  const [preview, setPreview] = useState<number | null>(null);
  const previewSlot = preview ?? connected[0] ?? null;

  // Ctrl+Z desfaz a última alteração do perfil (fora dos campos de texto).
  const { undo } = editor;
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "z" || event.shiftKey) {
        return;
      }
      const { target } = event;
      if (
        target instanceof Element &&
        target.closest("input, textarea, [contenteditable='true']")
      ) {
        return;
      }
      event.preventDefault();
      undo();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [undo]);

  const previewOptions: SelectOption[] = [
    { value: "none", label: t("preview.none") },
    ...[0, 1, 2, 3].map((n) => ({ value: String(n), label: t("preview.slot", { n: n + 1 }) })),
  ];
  const saveKey = SAVE_LABEL[saveState];

  const panel = (() => {
    const shared = { profile: draft.profile, readOnly, edit: editor.edit };
    if (element === null) return <p className="text-body text-fg-2">{t("panel.empty")}</p>;
    if (isButton(element)) return <ButtonPanel element={element} {...shared} />;
    if (element === "lt" || element === "rt") {
      return <TriggerPanel side={element === "lt" ? "left" : "right"} {...shared} />;
    }
    return <StickPanel side={element === "stick-left" ? "left" : "right"} {...shared} />;
  })();

  return (
    <>
      <section className="flex min-h-0 min-w-0 flex-col gap-3 overflow-auto">
        <div className="flex flex-wrap items-center gap-2">
          <ProfileIcon name={draft.icon} className="size-5 text-fg-2" />
          <Input
            aria-label={t("profiles.name")}
            value={draft.name}
            disabled={readOnly}
            maxLength={64}
            onChange={(e) => {
              editor.editMeta({ name: e.target.value });
            }}
            className="h-8 max-w-64 flex-1"
          />
          <Select
            label={t("profiles.icon")}
            value={draft.icon ?? "gamepad"}
            disabled={readOnly}
            className="min-w-28"
            options={PROFILE_ICON_NAMES.map((name) => ({
              value: name,
              label: (
                <span className="flex items-center gap-2">
                  <ProfileIcon name={name} className="size-4" />
                  {name}
                </span>
              ),
            }))}
            onValueChange={(icon) => {
              editor.editMeta({ icon });
            }}
          />
          <span className="flex-1" />
          {saveKey && (
            <span role="status" className="text-caption text-fg-3">
              {t(saveKey as "save.saved")}
            </span>
          )}
          <IconButton label={t("save.undo")} disabled={!canUndo || readOnly} onClick={editor.undo}>
            <Undo2 />
          </IconButton>
        </div>

        {readOnly && <Badge status="info">{t("save.readOnly")}</Badge>}

        <Card className="flex flex-1 flex-col items-center justify-center gap-4">
          <Gamepad
            mode="interactive"
            slot={previewSlot ?? undefined}
            color={slotColor(previewSlot ?? 0)}
            selected={element ?? undefined}
            onSelect={setElement}
            className="w-full max-w-xl"
          />
          <div className="flex items-center gap-2">
            <span className="text-caption text-fg-2">{t("preview.label")}</span>
            <Select
              label={t("preview.label")}
              value={preview === null ? "none" : String(preview)}
              options={previewOptions}
              className="min-w-24"
              onValueChange={(value) => {
                setPreview(value === "none" ? null : Number(value));
              }}
            />
          </div>
        </Card>
      </section>

      <aside aria-label={t("tabs.label")} className="flex min-h-0 flex-col gap-3 overflow-auto">
        <SegmentedControl
          label={t("tabs.label")}
          value={tab}
          onValueChange={setTab}
          options={[
            { value: "controls", label: t("tabs.controls") },
            { value: "gyro", label: t("tabs.gyro") },
            { value: "rumble", label: t("tabs.rumble") },
          ]}
        />
        <Card>
          {tab === "gyro" ? (
            <GyroPanel profile={draft.profile} readOnly={readOnly} edit={editor.edit} />
          ) : tab === "rumble" ? (
            <RumblePanel profile={draft.profile} readOnly={readOnly} edit={editor.edit} />
          ) : (
            panel
          )}
        </Card>
      </aside>
    </>
  );
}
