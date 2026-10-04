import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Markdown } from "@/components/Markdown";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { useAppInfo } from "@/lib/api/app";
import { useCheckForUpdate, useInstallUpdate, usePendingUpdate } from "@/lib/api/updates";
import { parseMarkdown } from "@/lib/markdown";
import { useToasts } from "@/lib/toast";

const noAnchor = () => undefined;

/**
 * Configurações → Sobre → Atualizações: procurar agora, ver as novidades e instalar. Com
 * controles conectados, instalar pede confirmação (o backend recusa sem ela, ver `updates.rs`).
 */
export function UpdatesControl() {
  const { t } = useTranslation("settings");
  const info = useAppInfo();
  const pending = usePendingUpdate();
  const check = useCheckForUpdate();
  const install = useInstallUpdate();
  const [showNotes, setShowNotes] = useState(false);
  /** Quantos controles estão conectados, quando falta a confirmação. */
  const [connected, setConnected] = useState<number | null>(null);

  const update = pending.data ?? null;
  const notes = useMemo(() => parseMarkdown(update?.notes ?? ""), [update?.notes]);
  const upToDate = !update && check.isSuccess;

  const run = (confirmed: boolean) => {
    install.mutate(confirmed, {
      onSuccess: (result) => {
        if (result.started) {
          useToasts.getState().push("info", t("update.installStarted"));
        } else {
          setConnected(result.active_sessions);
        }
      },
    });
  };

  return (
    <div className="flex flex-col items-end gap-3">
      <div className="flex flex-wrap items-center justify-end gap-3">
        {update && (
          <Badge status="info">{t("update.newVersion", { version: update.version })}</Badge>
        )}
        {upToDate && (
          <p className="text-body text-fg-2" role="status">
            {t("update.upToDate", { version: info.data?.app_version ?? "" })}
          </p>
        )}
        {update ? (
          <Button
            disabled={install.isPending}
            onClick={() => {
              run(false);
            }}
          >
            {install.isPending ? t("update.installing") : t("update.install")}
          </Button>
        ) : (
          <Button
            variant="secondary"
            disabled={check.isPending}
            onClick={() => {
              check.mutate();
            }}
          >
            {check.isPending ? t("update.checking") : t("update.check")}
          </Button>
        )}
      </div>

      {update && (
        <div className="flex w-full flex-col items-end gap-2">
          <Button
            variant="ghost"
            aria-expanded={showNotes}
            onClick={() => {
              setShowNotes((v) => !v);
            }}
          >
            {showNotes ? t("update.notesHide") : t("update.notesShow")}
          </Button>
          {showNotes && (
            <div className="w-full rounded-lg bg-control p-3 text-left select-text">
              {update.notes ? (
                <Markdown blocks={notes} onAnchor={noAnchor} />
              ) : (
                <p className="text-body text-fg-2">{t("update.noNotes")}</p>
              )}
            </div>
          )}
        </div>
      )}

      <ConfirmDialog
        open={connected !== null}
        onOpenChange={(open) => {
          if (!open) setConnected(null);
        }}
        title={t("update.confirmTitle")}
        description={t("update.confirmBody", { count: connected ?? 0 })}
        confirmLabel={t("update.confirmAction")}
        cancelLabel={t("update.cancel")}
        destructive
        onConfirm={() => {
          run(true);
        }}
      />
    </div>
  );
}
