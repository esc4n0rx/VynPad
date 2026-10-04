import { useEffect, useRef, useState, type ReactNode } from "react";
import { Check, Copy } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { useCancelPairing, useLastPaired, useStartPairing } from "@/lib/api/server";
import type { PairingOfferDto } from "@/lib/bindings";

/** Quanto o sucesso fica na tela antes de o modal fechar sozinho. */
export const SUCCESS_MS = 1800;

type Tab = "qr" | "pin";

interface PairDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Aparelho "novo" que o usuário escolheu na lista de próximos (só destaca no texto). */
  target?: string;
}

/** Modal de pareamento: o código só existe enquanto ele está aberto. */
export function PairDialog({ open, onOpenChange, target }: PairDialogProps) {
  const { t } = useTranslation("devices");
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogTitle>{t("pair.title")}</DialogTitle>
        <DialogDescription>
          {target ? t("pair.forDevice", { name: target }) : t("pair.description")}
        </DialogDescription>
        <PairingPanel
          onClose={() => {
            onOpenChange(false);
          }}
        />
      </DialogContent>
    </Dialog>
  );
}

interface PairingPanelProps {
  /** Pedido para fechar (o botão "Concluir" e, com `autoClose`, o fim da animação de sucesso). */
  onClose: () => void;
  /** Fecha sozinho depois do sucesso (o modal); a introdução segue pelo próprio botão. */
  autoClose?: boolean;
  /** Algo a mais embaixo da mensagem de sucesso (na introdução, o controle acendendo). */
  successExtra?: ReactNode;
  /** Sem o botão "Concluir" no sucesso. */
  hideDone?: boolean;
  /** Chamado uma vez quando um aparelho novo termina de parear. */
  onPaired?: (device: { device_id: string; name: string; model: string }) => void;
}

/**
 * O código de pareamento (QR e PIN) que se renova ao vencer, com a animação de sucesso quando um
 * aparelho novo conclui. O código só existe enquanto o painel está montado.
 */
export function PairingPanel({
  onClose,
  autoClose = true,
  successExtra,
  hideDone,
  onPaired,
}: PairingPanelProps) {
  const { t } = useTranslation("devices");
  const start = useStartPairing();
  const cancel = useCancelPairing();
  const [tab, setTab] = useState<Tab>("qr");
  const [offer, setOffer] = useState<PairingOfferDto | null>(null);
  const [issuedAt, setIssuedAt] = useState(0);
  const [now, setNow] = useState(0);
  const [baseline] = useState(() => useLastPaired.getState().seq);
  const paired = useLastPaired((s) => (s.seq > baseline ? s.last : null));
  const [copied, setCopied] = useState(false);
  const renewRef = useRef<(() => void) | null>(null);
  const { mutate: startPairing } = start;
  const { mutate: cancelPairing } = cancel;

  // Gera o código ao abrir e invalida ao fechar.
  useEffect(() => {
    const renew = () => {
      startPairing(undefined, {
        onSuccess: (next) => {
          const at = Date.now();
          setOffer(next);
          setIssuedAt(at);
          setNow(at);
        },
      });
    };
    renew();
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, 1000);
    renewRef.current = renew;
    return () => {
      window.clearInterval(timer);
      renewRef.current = null;
      cancelPairing();
    };
  }, [startPairing, cancelPairing]);

  const ttl = offer ? (tab === "qr" ? offer.token_ttl_s : offer.pin_ttl_s) : 0;
  const left = offer ? Math.max(0, Math.ceil(ttl - (now - issuedAt) / 1000)) : 0;

  // Venceu: pede outro (cada pedido invalida o PIN anterior).
  useEffect(() => {
    if (offer && left === 0 && !start.isPending && paired === null) renewRef.current?.();
  }, [offer, left, start.isPending, paired]);

  // Concluído: avisa quem quiser saber e, no modal, fecha sozinho depois do sucesso.
  const notified = useRef(false);
  useEffect(() => {
    if (paired === null) return;
    if (!notified.current) {
      notified.current = true;
      onPaired?.(paired);
    }
    if (!autoClose) return;
    const timer = window.setTimeout(onClose, SUCCESS_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [paired, onClose, autoClose, onPaired]);

  if (paired) {
    return (
      <div role="status" className="flex flex-col items-center gap-3 py-8">
        <motion.span
          initial={{ scale: 0.4, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          transition={{ type: "spring", stiffness: 380, damping: 22 }}
          className="flex size-16 items-center justify-center rounded-full bg-success/15 text-success"
        >
          <Check className="size-9" strokeWidth={2.25} />
        </motion.span>
        <p className="text-subtitle font-semibold text-fg">
          {t("pair.success", { name: paired.name })}
        </p>
        {successExtra}
        {!hideDone && (
          <Button variant="secondary" onClick={onClose}>
            {t("pair.done")}
          </Button>
        )}
      </div>
    );
  }

  if (start.isError && !offer) {
    return (
      <div className="flex flex-col items-center gap-3 py-8 text-center">
        <p className="text-body text-fg-2">{t("pair.error")}</p>
        <Button variant="secondary" onClick={() => renewRef.current?.()}>
          {t("pair.retry")}
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center gap-4">
      <SegmentedControl
        label={t("pair.tabs")}
        value={tab}
        onValueChange={setTab}
        options={[
          { value: "qr", label: t("pair.qr") },
          { value: "pin", label: t("pair.pin") },
        ]}
      />

      {offer === null ? (
        <p className="py-16 text-body text-fg-2">{t("pair.renewing")}</p>
      ) : tab === "qr" ? (
        <div
          role="img"
          aria-label={t("pair.qrAlt")}
          // O SVG é gerado pelo nosso backend (crate `qrcode`), nunca vem da rede.
          dangerouslySetInnerHTML={{ __html: offer.qr_svg }}
          className="size-56 rounded-lg bg-white p-2 [&_svg]:size-full"
        />
      ) : (
        <div className="flex flex-col items-center gap-2 py-6">
          <p
            aria-label={t("pair.pinLabel")}
            className="text-[40px] font-semibold tracking-[0.3em] text-fg tabular-nums select-text"
          >
            {offer.pin}
          </p>
          <p className="max-w-64 text-center text-body text-fg-2">{t("pair.pinHelp")}</p>
        </div>
      )}

      {offer && (
        <div className="flex w-full flex-col items-center gap-1 text-caption text-fg-3">
          <p className="tabular-nums">{t("pair.expires", { seconds: left })}</p>
          <p className="tabular-nums select-text">
            {t("pair.hosts", {
              hosts: offer.hosts.map((h) => `${h}:${String(offer.port)}`).join(", "),
            })}
          </p>
          <p className="tabular-nums select-text">
            {t("pair.fingerprint", { fingerprint: offer.fingerprint_short })}
          </p>
          <p>{t("pair.fingerprintHelp")}</p>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              void navigator.clipboard.writeText(offer.uri).then(() => {
                setCopied(true);
              });
            }}
          >
            {copied ? <Check /> : <Copy />}
            {copied ? t("pair.copied") : t("pair.copyUri")}
          </Button>
        </div>
      )}
    </div>
  );
}
