import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Toggle } from "@/components/ui/toggle";
import type { Settings } from "@/lib/api/settings";
import { SettingRow } from "./SettingRow";

interface SectionProps {
  settings: Settings;
  patch: (change: Partial<Settings>) => void;
}

function ServerName({ settings, patch }: SectionProps) {
  const { t } = useTranslation("settings");
  const [name, setName] = useState(settings.server_name);
  const save = () => {
    const next = name.trim();
    if (next !== settings.server_name) patch({ server_name: next });
  };

  return (
    <SettingRow id="serverName">
      {({ labelId, hintId }) => (
        <Input
          aria-labelledby={labelId}
          aria-describedby={hintId}
          value={name}
          maxLength={40}
          placeholder={t("general.serverNamePlaceholder")}
          onChange={(e) => {
            setName(e.target.value);
          }}
          onBlur={save}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
          className="w-64"
        />
      )}
    </SettingRow>
  );
}

function ToggleSetting({
  id,
  field,
  settings,
  patch,
}: SectionProps & {
  id: "autostart" | "startMinimized" | "closeToTray" | "requireApproval";
  field: "autostart" | "start_minimized" | "close_to_tray" | "require_approval";
}) {
  return (
    <SettingRow id={id}>
      {({ labelId, hintId }) => (
        <Toggle
          aria-labelledby={labelId}
          aria-describedby={hintId}
          checked={settings[field]}
          onCheckedChange={(checked) => {
            patch({ [field]: checked });
          }}
        />
      )}
    </SettingRow>
  );
}

/** Geral: nome do server, início com o sistema, bandeja, aprovação e a introdução. */
export function GeneralSection({ settings, patch }: SectionProps) {
  const { t } = useTranslation("settings");
  const navigate = useNavigate();

  return (
    <>
      {/* `key`: se o nome mudar por fora (outra janela), o campo recomeça do valor novo. */}
      <ServerName key={settings.server_name} settings={settings} patch={patch} />
      <ToggleSetting id="autostart" field="autostart" settings={settings} patch={patch} />
      <ToggleSetting
        id="startMinimized"
        field="start_minimized"
        settings={settings}
        patch={patch}
      />
      <ToggleSetting id="closeToTray" field="close_to_tray" settings={settings} patch={patch} />
      <ToggleSetting
        id="requireApproval"
        field="require_approval"
        settings={settings}
        patch={patch}
      />
      <SettingRow id="redoIntro">
        {() => (
          <Button
            variant="secondary"
            onClick={() => {
              patch({ onboarding_completed: false });
              void navigate("/onboarding");
            }}
          >
            {t("general.redoIntroButton")}
          </Button>
        )}
      </SettingRow>
    </>
  );
}
