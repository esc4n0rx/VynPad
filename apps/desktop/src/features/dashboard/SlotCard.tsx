import { useEffect, useMemo, useState, type DragEvent } from "react";
import {
  ArrowLeftRight,
  Activity,
  MoreHorizontal,
  Pause,
  Play,
  Plus,
  Power,
  Smartphone,
} from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { Gamepad } from "@/components/gamepad/Gamepad";
import { slotColor } from "@/components/gamepad/elements";
import { DeviceIcon } from "@/components/icons/DeviceIcon";
import { SlotBadge } from "@/components/icons/SlotBadge";
import { ActionMenu, type ActionMenuItem } from "@/components/ui/action-menu";
import { Badge } from "@/components/ui/badge";
import { Button, IconButton } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Select, type SelectOption } from "@/components/ui/select";
import { useProfile } from "@/lib/api/profiles";
import type { DeviceDto, ProfileSummaryDto, SessionHealthDto, SlotDto } from "@/lib/bindings";
import { cn } from "@/lib/utils";
import { secondsLeft, type SlotCardKind } from "./slotView";

export const DRAG_TYPE = "application/x-padgaming-slot";

interface SlotCardProps {
  slot: SlotDto;
  kind: SlotCardKind;
  health: SessionHealthDto | undefined;
  device: DeviceDto | undefined;
  profiles: readonly ProfileSummaryDto[];
  /** `id` do perfil que o slot está usando. */
  profileId: string | null;
  onPair: () => void;
  onProfile: (profileId: string) => void;
  onPause: (paused: boolean) => void;
  onDisconnect: () => void;
  onLatency: () => void;
  /** Troca este slot com `other` (menu ou arrastar e soltar). */
  onSwap: (other: number) => void;
}

/** Contagem regressiva da reserva: o backend manda o tempo que falta quando o estado muda. */
function Countdown({ expiresInMs }: { expiresInMs: number }) {
  const { t } = useTranslation("dashboard");
  const [deadline] = useState(() => Date.now() + expiresInMs);
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
    <span className="tabular-nums">
      {" · "}
      {t("slot.reconnecting.countdown", { seconds: secondsLeft(deadline, now) })}
    </span>
  );
}

function EmptyCard({ slot, onPair }: Pick<SlotCardProps, "slot" | "onPair">) {
  const { t } = useTranslation("dashboard");
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 text-center">
      <Gamepad
        mode="mini"
        report={undefined}
        color={slotColor(slot.slot)}
        className="w-44 opacity-40 [&_.pad-body]:[stroke-dasharray:5_4]"
      />
      <div>
        <p className="text-subtitle font-semibold text-fg">{t("slot.empty.title")}</p>
        <p className="text-body text-fg-2">{t("slot.empty.hint")}</p>
      </div>
      <Button variant="secondary" onClick={onPair}>
        <Plus />
        {t("slot.empty.pair")}
      </Button>
    </div>
  );
}

function PhysicalCard({ slot }: Pick<SlotCardProps, "slot">) {
  const { t } = useTranslation("dashboard");
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 text-center">
      <Gamepad mode="mini" color="var(--text-tertiary)" className="w-44 opacity-40" />
      <div>
        <p className="text-subtitle font-semibold text-fg">{t("slot.physical.title")}</p>
        <p className="mx-auto max-w-64 text-body text-fg-2">
          {t("slot.physical.hint", { slot: slot.slot + 1 })}
        </p>
      </div>
    </div>
  );
}

export function SlotCard(props: SlotCardProps) {
  const { slot, kind, health, device, profiles, profileId } = props;
  const { t } = useTranslation("dashboard");
  const [dragOver, setDragOver] = useState(false);
  const profile = useProfile(kind === "connected" || kind === "reconnecting" ? profileId : null);
  const occupied = kind === "connected" || kind === "reconnecting";
  const number = slot.slot + 1;

  const options = useMemo<SelectOption[]>(
    () => profiles.map((p) => ({ value: p.id, label: p.name })),
    [profiles],
  );

  const menu: ActionMenuItem[] = [
    {
      key: "swap",
      label: t("actions.swap"),
      icon: <ArrowLeftRight />,
      items: [0, 1, 2, 3]
        .filter((other) => other !== slot.slot)
        .map((other) => ({
          key: `swap-${String(other)}`,
          label: t("actions.swapWith", { slot: other + 1 }),
          onSelect: () => {
            props.onSwap(other);
          },
        })),
    },
    {
      key: "pause",
      label: slot.paused ? t("actions.resume") : t("actions.pause"),
      icon: slot.paused ? <Play /> : <Pause />,
      onSelect: () => {
        props.onPause(!slot.paused);
      },
    },
    {
      key: "latency",
      label: t("actions.latency"),
      icon: <Activity />,
      onSelect: props.onLatency,
    },
    {
      key: "disconnect",
      label: t("actions.disconnect"),
      icon: <Power />,
      destructive: true,
      separated: true,
      onSelect: props.onDisconnect,
    },
  ];

  const onDragStart = (event: DragEvent) => {
    event.dataTransfer.setData(DRAG_TYPE, String(slot.slot));
    event.dataTransfer.effectAllowed = "move";
  };
  const onDragOver = (event: DragEvent) => {
    if (!event.dataTransfer.types.includes(DRAG_TYPE)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    setDragOver(true);
  };
  const onDrop = (event: DragEvent) => {
    setDragOver(false);
    const from = Number(event.dataTransfer.getData(DRAG_TYPE));
    if (!Number.isInteger(from) || from === slot.slot) return;
    event.preventDefault();
    props.onSwap(from);
  };

  const reconnecting = kind === "reconnecting";
  const deviceName = device?.name ?? t("slot.unknownDevice");

  return (
    <Card
      data-testid={`slot-${String(slot.slot)}`}
      data-kind={kind}
      role="group"
      aria-label={t("slot.group", { slot: number })}
      draggable={occupied}
      onDragStart={onDragStart}
      onDragEnd={() => {
        setDragOver(false);
      }}
      onDragOver={onDragOver}
      onDragLeave={() => {
        setDragOver(false);
      }}
      onDrop={onDrop}
      title={occupied ? t("slot.dragHint") : undefined}
      className={cn(
        "relative flex min-h-72 flex-col gap-3 overflow-hidden p-4 transition-shadow",
        occupied && "cursor-grab active:cursor-grabbing",
        dragOver && "ring-2 ring-accent",
      )}
    >
      {kind === "connected" && slot.session !== null && (
        // "Acende" ao conectar: um anel na cor do slot que cresce e some.
        <motion.span
          key={slot.session}
          aria-hidden
          initial={{ opacity: 0.9, scale: 0.92 }}
          animate={{ opacity: 0, scale: 1.04 }}
          transition={{ duration: 0.9, ease: "easeOut" }}
          style={{ boxShadow: `inset 0 0 0 2px ${slotColor(slot.slot)}` }}
          className="pointer-events-none absolute inset-0 rounded-lg"
        />
      )}

      <header className="flex items-center gap-2">
        <SlotBadge slot={slot.slot} dim={!occupied} />
        {occupied && (
          <div className="flex min-w-0 flex-1 items-center gap-2">
            <DeviceIcon
              status={reconnecting ? "idle" : "connected"}
              decorative
              className="size-6"
            />
            <div className="min-w-0">
              <p className="truncate text-body font-medium text-fg">{deviceName}</p>
              {device?.model && <p className="truncate text-caption text-fg-3">{device.model}</p>}
            </div>
          </div>
        )}
        {!occupied && <span className="flex-1" />}
        {occupied && (
          <ActionMenu
            trigger={
              <IconButton label={t("slot.menu", { slot: number })} size="sm">
                <MoreHorizontal />
              </IconButton>
            }
            items={menu}
          />
        )}
      </header>

      {kind === "empty" && <EmptyCard slot={slot} onPair={props.onPair} />}
      {kind === "physical" && <PhysicalCard slot={slot} />}

      {occupied && (
        <>
          <div className="flex flex-1 items-center justify-center">
            <Gamepad
              mode="live"
              slot={slot.slot}
              color={slotColor(slot.slot)}
              className={cn("w-full max-w-72 transition-opacity", reconnecting && "opacity-40")}
            />
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {reconnecting && (
              <Badge status="warning" className="animate-pulse">
                {health === "stalled" && slot.state !== "suspended"
                  ? t("slot.stalled")
                  : t("slot.reconnecting.title")}
                {slot.expires_in_ms !== null && (
                  <Countdown key={slot.expires_in_ms} expiresInMs={slot.expires_in_ms} />
                )}
              </Badge>
            )}
            {!reconnecting && slot.paused && <Badge status="info">{t("slot.paused")}</Badge>}
            {profile.data?.profile.gyro?.enabled === true && (
              <Badge status="neutral">
                <Smartphone aria-hidden className="size-3" />
                {t("slot.gyro")}
              </Badge>
            )}
            <span className="flex-1" />
            <Select
              label={t("slot.profile", { slot: number })}
              placeholder={t("slot.profilePlaceholder")}
              options={options}
              value={profileId ?? ""}
              onValueChange={props.onProfile}
              className="min-w-40"
            />
          </div>
        </>
      )}
    </Card>
  );
}
