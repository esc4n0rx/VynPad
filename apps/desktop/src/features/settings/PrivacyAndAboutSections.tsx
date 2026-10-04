import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Toggle } from "@/components/ui/toggle";
import { useNavigate } from "react-router";
import { useAppInfo } from "@/lib/api/app";
import type { Settings } from "@/lib/api/settings";
import {
  useClearHistory,
  useExportDiagnostics,
  useLogsDir,
  useOpenLogsDir,
  useRegenerateIdentity,
  useRestartApp,
} from "@/lib/api/system";
import { useToasts } from "@/lib/toast";
import notices from "../../../../../docs/THIRD-PARTY-NOTICES.md?raw";
import { SettingRow } from "./SettingRow";

interface SectionProps {
  settings: Settings;
  patch: (change: Partial<Settings>) => void;
}

const LANGUAGES = ["pt-BR", "en"] as const;

/** Idioma do app (o texto de cada idioma fica no próprio idioma). */
export function LanguageSection({ settings, patch }: SectionProps) {
  const { t } = useTranslation("settings");
  return (
    <SettingRow id="language">
      {() => (
        <SegmentedControl
          label={t("items.language.label")}
          value={settings.language}
          options={LANGUAGES.map((language) => ({
            value: language,
            label: t(`language.${language}`),
          }))}
          onValueChange={(language) => {
            patch({ language });
          }}
        />
      )}
    </SettingRow>
  );
}

/** Privacidade e segurança: limpar o histórico e regenerar a identidade do server. */
export function PrivacySection() {
  const { t } = useTranslation("settings");
  const clear = useClearHistory();
  const regenerate = useRegenerateIdentity();
  const restart = useRestartApp();
  const [dialog, setDialog] = useState<"clear" | "regenerate" | "restart" | null>(null);
  const close = (open: boolean) => {
    if (!open) setDialog(null);
  };

  return (
    <>
      <SettingRow id="clearHistory">
        {() => (
          <Button
            variant="secondary"
            onClick={() => {
              setDialog("clear");
            }}
          >
            {t("privacy.clear.button")}
          </Button>
        )}
      </SettingRow>
      <SettingRow id="regenerateIdentity">
        {() => (
          <Button
            variant="destructive"
            onClick={() => {
              setDialog("regenerate");
            }}
          >
            {t("privacy.regenerate.button")}
          </Button>
        )}
      </SettingRow>

      <ConfirmDialog
        open={dialog === "clear"}
        onOpenChange={close}
        title={t("privacy.clear.title")}
        description={t("privacy.clear.body")}
        confirmLabel={t("privacy.clear.confirm")}
        cancelLabel={t("privacy.cancel")}
        destructive
        onConfirm={() => {
          clear.mutate(undefined, {
            onSuccess: (count) => {
              useToasts.getState().push("success", t("privacy.clear.done", { count }));
            },
          });
        }}
      />
      <ConfirmDialog
        open={dialog === "regenerate"}
        onOpenChange={close}
        title={t("privacy.regenerate.title")}
        description={t("privacy.regenerate.body")}
        confirmLabel={t("privacy.regenerate.confirm")}
        cancelLabel={t("privacy.cancel")}
        destructive
        onConfirm={() => {
          regenerate.mutate(undefined, {
            onSuccess: () => {
              useToasts.getState().push("success", t("privacy.regenerate.done"));
              setDialog("restart");
            },
          });
        }}
      />
      <ConfirmDialog
        open={dialog === "restart"}
        onOpenChange={close}
        title={t("privacy.regenerate.restartTitle")}
        description={t("privacy.regenerate.restartBody")}
        confirmLabel={t("privacy.regenerate.restartNow")}
        cancelLabel={t("privacy.regenerate.later")}
        onConfirm={() => {
          restart.mutate();
        }}
      />
    </>
  );
}

/** Sobre: versão, pasta de logs, atualizações, diagnóstico e licenças. */
export function AboutSection() {
  const { t } = useTranslation("settings");
  const info = useAppInfo();
  const logs = useLogsDir();
  const openLogs = useOpenLogsDir();
  const diagnostics = useExportDiagnostics();
  const navigate = useNavigate();
  const [maskIps, setMaskIps] = useState(false);
  const [showLicenses, setShowLicenses] = useState(false);

  return (
    <>
      <SettingRow id="version">
        {() => (
          <p className="text-body text-fg-2 tabular-nums select-text">
            {info.data
              ? t("about.version", { app: info.data.app_version, core: info.data.core_version })
              : "—"}
          </p>
        )}
      </SettingRow>
      <SettingRow id="logs">
        {() => (
          <div className="flex flex-col items-end gap-1">
            <Button
              variant="secondary"
              onClick={() => {
                openLogs.mutate();
              }}
            >
              {t("about.openLogs")}
            </Button>
            {logs.data && (
              <p className="max-w-xs truncate text-caption text-fg-3 select-text" title={logs.data}>
                {t("about.logsPath", { path: logs.data })}
              </p>
            )}
          </div>
        )}
      </SettingRow>
      <SettingRow id="updates">
        {() => (
          <Button variant="secondary" disabled>
            {t("about.soon")}
          </Button>
        )}
      </SettingRow>
      <SettingRow id="help">
        {() => (
          <Button
            variant="secondary"
            onClick={() => {
              void navigate("/help");
            }}
          >
            {t("about.openHelp")}
          </Button>
        )}
      </SettingRow>
      <SettingRow id="diagnostic" stacked>
        {() => (
          <div className="flex flex-wrap items-center gap-4">
            <Button
              variant="secondary"
              disabled={diagnostics.isPending}
              onClick={() => {
                diagnostics.mutate(maskIps, {
                  onSuccess: (path) => {
                    useToasts.getState().push("success", t("about.exported", { path }));
                  },
                });
              }}
            >
              {diagnostics.isPending ? t("about.exporting") : t("about.exportButton")}
            </Button>
            <div className="flex items-center gap-2">
              <Toggle
                aria-labelledby="mask-ips-label"
                checked={maskIps}
                onCheckedChange={setMaskIps}
              />
              <span id="mask-ips-label" className="text-body text-fg-2">
                {t("about.maskIps")}
              </span>
            </div>
          </div>
        )}
      </SettingRow>
      <SettingRow id="licenses" stacked>
        {() => (
          <div className="flex flex-col gap-2">
            <div>
              <Button
                variant="secondary"
                aria-expanded={showLicenses}
                onClick={() => {
                  setShowLicenses((v) => !v);
                }}
              >
                {showLicenses ? t("about.licensesHide") : t("about.licensesShow")}
              </Button>
            </div>
            {showLicenses && (
              <textarea
                readOnly
                aria-label={t("items.licenses.label")}
                value={notices}
                rows={14}
                className="w-full resize-y rounded-lg bg-control p-3 text-caption text-fg-2 focus-ring select-text"
              />
            )}
          </div>
        )}
      </SettingRow>
    </>
  );
}
