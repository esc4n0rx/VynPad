import { Toggle } from "@/components/ui/toggle";
import type { Settings } from "@/lib/api/settings";
import { SettingRow } from "./SettingRow";
import type { ItemId } from "./settingsIndex";

interface SectionProps {
  settings: Settings;
  patch: (change: Partial<Settings>) => void;
}

const TOGGLES: readonly {
  id: ItemId;
  field: "notify_connections" | "notify_pairing" | "notify_driver" | "notify_skip_fullscreen";
}[] = [
  { id: "notifyConnections", field: "notify_connections" },
  { id: "notifyPairing", field: "notify_pairing" },
  { id: "notifyDriver", field: "notify_driver" },
  { id: "notifySkipFullscreen", field: "notify_skip_fullscreen" },
];

/** Notificações: quais avisos aparecem (toast na janela ou notificação do sistema). */
export function NotificationsSection({ settings, patch }: SectionProps) {
  return (
    <>
      {TOGGLES.map(({ id, field }) => (
        <SettingRow key={id} id={id}>
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
      ))}
    </>
  );
}
