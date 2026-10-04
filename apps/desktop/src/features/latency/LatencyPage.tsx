import { useMemo, useState } from "react";
import { Activity, Check, Copy, Pause, Play } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SlotBadge } from "@/components/icons/SlotBadge";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { formatNumber } from "@/i18n/format";
import { useMetrics } from "@/lib/api/metrics";
import { SLOT_COUNT } from "@/lib/api/latencyHistory";
import type { QualityDto, SlotMetricsDto } from "@/lib/bindings";
import { cn } from "@/lib/utils";
import { LatencyChart, type WindowSeconds } from "./LatencyChart";
import { summarize, tipsFor } from "./insights";

const QUALITY_STATUS = {
  excellent: "success",
  good: "info",
  unstable: "warning",
  poor: "danger",
} as const satisfies Record<QualityDto, string>;

const SLOT_BG = ["bg-slot-1", "bg-slot-2", "bg-slot-3", "bg-slot-4"] as const;

function Tile({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col gap-0.5">
      <dt className="text-caption text-fg-3">{label}</dt>
      <dd className="text-subtitle font-semibold text-fg tabular-nums">{value}</dd>
    </div>
  );
}

function SlotTiles({ metrics }: { metrics: SlotMetricsDto }) {
  const { t } = useTranslation("latency");
  const ms = (value: number | null, digits = 1) =>
    t("tiles.ms", {
      value: formatNumber(value ?? 0, {
        minimumFractionDigits: digits,
        maximumFractionDigits: digits,
      }),
    });
  return (
    <Card
      role="group"
      aria-label={t("tiles.group", { slot: metrics.slot + 1 })}
      className="flex flex-col gap-3"
    >
      <div className="flex items-center gap-2">
        <SlotBadge slot={metrics.slot} />
        <span className="flex-1" />
        {metrics.quality === null ? (
          <Badge>{t("quality.waiting")}</Badge>
        ) : (
          <Badge status={QUALITY_STATUS[metrics.quality]}>{t(`quality.${metrics.quality}`)}</Badge>
        )}
      </div>
      <dl className="grid grid-cols-2 gap-x-4 gap-y-3 sm:grid-cols-3">
        <Tile
          label={t("tiles.rtt")}
          value={metrics.rtt_ms === null ? t("tiles.none") : ms(metrics.rtt_percentiles_ms.p95)}
        />
        <Tile label={t("tiles.jitter")} value={ms(metrics.jitter_ms)} />
        <Tile
          label={t("tiles.loss")}
          value={t("tiles.pct", {
            value: formatNumber(metrics.loss_pct ?? 0, {
              maximumFractionDigits: 1,
              minimumFractionDigits: 1,
            }),
          })}
        />
        <Tile
          label={t("tiles.rate")}
          value={t("tiles.hz", {
            value: formatNumber(metrics.rate_hz ?? 0, { maximumFractionDigits: 0 }),
          })}
        />
        <Tile label={t("tiles.processing")} value={ms(metrics.processing_percentiles_ms.p95, 2)} />
      </dl>
    </Card>
  );
}

/** Se a rede está boa para jogar e, quando não está, por quê. */
export function LatencyPage() {
  const { t, i18n: language } = useTranslation("latency");
  const bySlot = useMetrics((s) => s.bySlot);
  const [windowSec, setWindowSec] = useState<WindowSeconds>(120);
  const [frozen, setFrozen] = useState(false);
  const [hidden, setHidden] = useState<ReadonlySet<number>>(new Set());
  const [copied, setCopied] = useState(false);

  const connected = useMemo(
    () =>
      Object.values(bySlot)
        .filter((m): m is SlotMetricsDto => m !== undefined)
        .sort((a, b) => a.slot - b.slot),
    [bySlot],
  );
  const tips = connected.flatMap((m) => tipsFor(m).map((tip) => ({ slot: m.slot, tip })));

  const toggleSlot = (slot: number) => {
    setHidden((current) => {
      const next = new Set(current);
      if (!next.delete(slot)) next.add(slot);
      return next;
    });
  };

  const copySummary = () => {
    const text = summarize(
      connected,
      {
        slot: (slot) => t("summary.slot", { slot: slot + 1 }),
        quality: (m) =>
          t("summary.quality", {
            label: t(m.quality === null ? "quality.waiting" : `quality.${m.quality}`),
          }),
        line: (values) => t("summary.line", values),
      },
      language.language,
    );
    void navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      window.setTimeout(() => {
        setCopied(false);
      }, 2000);
    });
  };

  return (
    <div className="flex min-h-full flex-col gap-4 p-6">
      <header className="flex flex-wrap items-center gap-3">
        <h1 className="flex-1 text-title font-semibold tracking-tight">{t("title")}</h1>
        <SegmentedControl
          label={t("windows.label")}
          value={String(windowSec)}
          onValueChange={(value) => {
            setWindowSec(Number(value) as WindowSeconds);
          }}
          options={[
            { value: "30", label: t("windows.s30") },
            { value: "120", label: t("windows.m2") },
            { value: "600", label: t("windows.m10") },
          ]}
        />
        <Button
          variant="secondary"
          aria-pressed={frozen}
          onClick={() => {
            setFrozen((v) => !v);
          }}
        >
          {frozen ? <Play /> : <Pause />}
          {frozen ? t("resume") : t("pause")}
        </Button>
        <Button variant="secondary" disabled={connected.length === 0} onClick={copySummary}>
          {copied ? <Check /> : <Copy />}
          {copied ? t("copied") : t("copy")}
        </Button>
      </header>

      <Card className="flex flex-col gap-3">
        <LatencyChart windowSec={windowSec} frozen={frozen} hidden={hidden} />
        <div
          className="flex flex-wrap items-center gap-2"
          role="group"
          aria-label={t("legend.label")}
        >
          {Array.from({ length: SLOT_COUNT }, (_, slot) => {
            const shown = !hidden.has(slot);
            return (
              <button
                key={slot}
                type="button"
                aria-pressed={shown}
                aria-label={t("legend.toggle", { slot: slot + 1 })}
                onClick={() => {
                  toggleSlot(slot);
                }}
                className={cn(
                  "flex items-center gap-1.5 rounded-full bg-control px-2.5 py-1 text-caption text-fg focus-ring transition-opacity hover:bg-control-hover",
                  !shown && "opacity-45",
                )}
              >
                <span aria-hidden className={cn("size-2.5 rounded-full", SLOT_BG[slot])} />
                {`P${String(slot + 1)}`}
              </button>
            );
          })}
          {frozen && <Badge status="info">{t("paused")}</Badge>}
        </div>
      </Card>

      {connected.length === 0 ? (
        <EmptyState
          icon={<Activity strokeWidth={1.5} />}
          title={t("empty.title")}
          description={t("empty.description")}
        />
      ) : (
        <div className="grid grid-cols-1 gap-4 min-[860px]:grid-cols-2">
          {connected.map((m) => (
            <SlotTiles key={m.slot} metrics={m} />
          ))}
        </div>
      )}

      {tips.length > 0 && (
        <section aria-labelledby="tips-title" className="flex flex-col gap-2">
          <h2 id="tips-title" className="text-subtitle font-semibold text-fg">
            {t("tips.title")}
          </h2>
          <ul className="flex flex-col gap-1 text-body text-fg-2">
            {tips.map(({ slot, tip }) => (
              <li key={`${String(slot)}-${tip}`}>{t(`tips.${tip}`, { slot: slot + 1 })}</li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
