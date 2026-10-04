import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { useAnswerApproval, useApprovals } from "@/lib/api/server";
import type { ApprovalPromptDto } from "@/lib/bindings";

function Countdown({ seconds }: { seconds: number }) {
  const { t } = useTranslation("devices");
  const [deadline] = useState(() => Date.now() + seconds * 1000);
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => {
      window.clearInterval(timer);
    };
  }, []);
  return (
    <p className="text-caption text-fg-3 tabular-nums">
      {t("approval.expires", { seconds: Math.max(0, Math.ceil((deadline - now) / 1000)) })}
    </p>
  );
}

/**
 * "Fulano quer se conectar" (com `require_approval` ligado). Fica na raiz do app: o pedido chega
 * em qualquer tela, e fechar o diálogo sem responder equivale a negar. Um pedido por vez.
 */
export function ApprovalDialog() {
  const { t } = useTranslation("devices");
  const prompt: ApprovalPromptDto | undefined = useApprovals((s) => s.pending[0]);
  const answer = useAnswerApproval();

  const respond = (accept: boolean) => {
    if (prompt) answer.mutate({ requestId: prompt.request_id, accept });
  };

  return (
    <Dialog
      open={prompt !== undefined}
      onOpenChange={(open) => {
        if (!open) respond(false);
      }}
    >
      {prompt && (
        <DialogContent key={prompt.request_id}>
          <DialogTitle>{t("approval.title", { name: prompt.name })}</DialogTitle>
          <DialogDescription>
            {t("approval.body", { model: prompt.model, ip: prompt.peer })}
          </DialogDescription>
          <Countdown seconds={prompt.timeout_s} />
          <DialogFooter>
            <Button
              variant="secondary"
              onClick={() => {
                respond(false);
              }}
            >
              {t("approval.deny")}
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                respond(true);
              }}
            >
              {t("approval.allow")}
            </Button>
          </DialogFooter>
        </DialogContent>
      )}
    </Dialog>
  );
}
