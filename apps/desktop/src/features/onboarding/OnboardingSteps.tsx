import { Check, Download, Network, ShieldAlert, Sparkles } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { Gamepad } from "@/components/gamepad/Gamepad";
import { slotColor } from "@/components/gamepad/elements";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Toggle } from "@/components/ui/toggle";
import { PairingPanel } from "@/features/devices/PairDialog";
import { messageForError } from "@/i18n/errors";
import { useDevices } from "@/lib/api/devices";
import { useDriverStatus, useNearbyDevices, useServerInfo } from "@/lib/api/server";
import type { Settings } from "@/lib/api/settings";
import {
  isCancelled,
  useFirewallStatus,
  useFixFirewall,
  useOpenNetworkSettings,
  useOpenVigemDownload,
} from "@/lib/api/system";
import { useToasts } from "@/lib/toast";

interface SettingsProps {
  settings: Settings;
  patch: (change: Partial<Settings>) => void;
}

const LANGUAGES = ["pt-BR", "en"] as const;

function StepHeading({ title, description }: { title: string; description?: string }) {
  return (
    <header className="flex flex-col gap-2 text-center">
      <h2 className="text-[26px] font-semibold tracking-tight text-fg">{title}</h2>
      {description && <p className="mx-auto max-w-lg text-body text-fg-2">{description}</p>}
    </header>
  );
}

// ------------------------------------------------------------------ 1. boas-vindas

export function WelcomeStep({ settings, patch }: SettingsProps) {
  const { t } = useTranslation("onboarding");
  const { t: ts } = useTranslation("settings");
  return (
    <div className="flex flex-col items-center gap-6">
      <motion.div
        initial={{ opacity: 0, scale: 0.85 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ type: "spring", stiffness: 300, damping: 26 }}
        className="flex size-24 items-center justify-center rounded-[28px] bg-gradient-to-br from-accent to-indigo shadow-lg"
      >
        <Sparkles aria-hidden className="size-12 text-on-accent" strokeWidth={1.5} />
      </motion.div>
      <StepHeading title={t("welcome.title")} description={t("welcome.description")} />
      <SegmentedControl
        label={t("welcome.language")}
        value={settings.language}
        options={LANGUAGES.map((language) => ({
          value: language,
          // Cada idioma aparece no próprio idioma, em qualquer idioma da interface.
          label: ts(`language.${language}`),
        }))}
        onValueChange={(language) => {
          patch({ language });
        }}
      />
    </div>
  );
}

// ------------------------------------------------------------------ 2. driver

export function DriverStep() {
  const { t } = useTranslation("onboarding");
  const driver = useDriverStatus();
  const download = useOpenVigemDownload();
  const status = driver.data;

  const message = !status
    ? t("driver.checking")
    : status.state === "ready"
      ? t("driver.ready")
      : status.state === "not_installed"
        ? t("driver.missing")
        : status.state === "outdated"
          ? t("driver.outdated", { found: status.found, required: status.required })
          : status.state === "service_stopped"
            ? t("driver.stopped")
            : t("driver.error", { message: status.message });
  const ready = status?.state === "ready";
  const needsDownload = status?.state === "not_installed" || status?.state === "outdated";

  return (
    <div className="flex flex-col items-center gap-5">
      <StepHeading title={t("driver.title")} description={t("driver.what")} />
      <div role="status" className="flex items-center gap-3">
        <Badge status={ready ? "success" : status ? "warning" : "neutral"}>{message}</Badge>
      </div>
      {needsDownload && (
        <div className="flex flex-col items-center gap-2">
          <Button
            variant="primary"
            onClick={() => {
              download.mutate();
            }}
          >
            <Download />
            {t("driver.download")}
          </Button>
          <p className="text-caption text-fg-3">{t("driver.afterInstall")}</p>
        </div>
      )}
      {status && !ready && (
        <p className="max-w-md text-center text-caption text-fg-3">{t("driver.warning")}</p>
      )}
    </div>
  );
}

// ------------------------------------------------------------------ 3. rede

export function NetworkStep() {
  const { t } = useTranslation("onboarding");
  const server = useServerInfo();
  const firewall = useFirewallStatus();
  const fix = useFixFirewall();
  const openNetwork = useOpenNetworkSettings();
  const status = firewall.data;
  const address = server.data?.addresses[0];

  const summary = firewall.isError
    ? t("network.failed")
    : !status
      ? t("driver.checking")
      : !status.applicable
        ? t("network.notApplicable")
        : !status.firewall_on
          ? t("network.firewallOff")
          : status.ok
            ? t("network.firewallOk")
            : t("network.firewallBlocked");
  const good = status?.ok === true;

  return (
    <div className="flex flex-col items-center gap-5">
      <StepHeading title={t("network.title")} description={t("network.description")} />

      <dl className="grid w-full max-w-sm grid-cols-[auto_1fr] gap-x-4 gap-y-1 rounded-lg bg-control p-4 text-body">
        <dt className="text-fg-3">{t("network.name")}</dt>
        <dd className="text-fg select-text">{server.data?.name ?? "—"}</dd>
        <dt className="text-fg-3">{t("network.address")}</dt>
        <dd className="text-fg tabular-nums select-text">
          {address
            ? `${address.ip}:${String(server.data?.control_port ?? "")}`
            : t("network.noAddress")}
        </dd>
      </dl>

      <div role="status" className="flex flex-wrap items-center justify-center gap-3">
        <Badge status={good ? "success" : status ? "warning" : "neutral"}>
          {good ? (
            <Check aria-hidden className="size-3" />
          ) : (
            <Network aria-hidden className="size-3" />
          )}
          {summary}
        </Badge>
        {status?.applicable && !status.ok && (
          <Button
            variant="primary"
            size="sm"
            disabled={fix.isPending}
            onClick={() => {
              fix.mutate(undefined, {
                onSuccess: () => {
                  useToasts.getState().push("success", t("network.allowed"));
                },
                onError: (error) => {
                  useToasts
                    .getState()
                    .push(
                      "error",
                      isCancelled(error) ? t("network.cancelled") : messageForError(error),
                    );
                },
              });
            }}
          >
            {fix.isPending ? t("network.allowing") : t("network.allow")}
          </Button>
        )}
        {(firewall.isError || (status && !status.ok)) && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              void firewall.refetch();
            }}
          >
            {t("network.recheck")}
          </Button>
        )}
      </div>

      {status?.public_network && (
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Badge status="warning">
            <ShieldAlert aria-hidden className="size-3" />
            {t("network.public")}
          </Badge>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              openNetwork.mutate();
            }}
          >
            {t("network.openNetwork")}
          </Button>
        </div>
      )}
    </div>
  );
}

// ------------------------------------------------------------------ 4. celular

export function ConnectStep({ onPaired }: { onPaired: () => void }) {
  const { t } = useTranslation("onboarding");
  const { t: td } = useTranslation("devices");
  const nearby = useNearbyDevices();
  const list = nearby.data ?? [];

  return (
    <div className="flex flex-col items-center gap-5">
      <StepHeading title={t("connect.title")} description={t("connect.description")} />
      <PairingPanel
        autoClose={false}
        hideDone
        onClose={() => undefined}
        onPaired={onPaired}
        successExtra={
          <div className="flex flex-col items-center gap-2">
            <Gamepad mode="live" slot={0} color={slotColor(0)} className="w-56" />
            <p className="text-caption text-fg-2">{t("connect.successHint")}</p>
          </div>
        }
      />
      <section aria-label={t("connect.nearby")} className="flex w-full max-w-sm flex-col gap-1">
        <h3 className="text-caption font-medium text-fg-2">{t("connect.nearby")}</h3>
        {list.length === 0 ? (
          <p className="text-caption text-fg-3">{t("connect.nearbyEmpty")}</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {list.map((device) => (
              <li key={device.id} className="flex items-center gap-2 text-body text-fg">
                <span className="flex-1 truncate">{device.name}</span>
                <span className="text-caption text-fg-3 tabular-nums">{device.ip}</span>
                <Badge status={device.paired ? "neutral" : "info"}>
                  {device.paired ? td("nearby.state.paired") : td("nearby.state.new")}
                </Badge>
              </li>
            ))}
          </ul>
        )}
      </section>
      <p className="text-caption text-fg-3">{t("connect.getApp")}</p>
    </div>
  );
}

// ------------------------------------------------------------------ 5. pronto

export function DoneStep({ settings, patch }: SettingsProps) {
  const { t } = useTranslation("onboarding");
  const driver = useDriverStatus().data;
  const firewall = useFirewallStatus().data;
  const devices = useDevices().data ?? [];
  const paired = devices.filter((d) => !d.revoked).length;

  const rows: { label: string; ok: boolean; value: string }[] = [
    {
      label: t("done.driver"),
      ok: driver?.state === "ready",
      value: driver?.state === "ready" ? t("done.ok") : t("done.pending"),
    },
    {
      label: t("done.network"),
      ok: firewall?.ok === true,
      value: firewall?.ok === true ? t("done.ok") : t("done.pending"),
    },
    { label: t("done.devices"), ok: paired > 0, value: String(paired) },
  ];

  return (
    <div className="flex flex-col items-center gap-5">
      <StepHeading title={t("done.title")} />
      <ul
        aria-label={t("done.summary")}
        className="flex w-full max-w-sm flex-col gap-1 rounded-lg bg-control p-4"
      >
        {rows.map((row) => (
          <li key={row.label} className="flex items-center gap-2 text-body text-fg">
            <span className="flex-1">{row.label}</span>
            <Badge status={row.ok ? "success" : "warning"}>{row.value}</Badge>
          </li>
        ))}
      </ul>

      <div className="flex w-full max-w-sm flex-col gap-3">
        <div className="flex items-center justify-between gap-3">
          <span id="ob-autostart" className="text-body text-fg">
            {t("done.autostart")}
          </span>
          <Toggle
            aria-labelledby="ob-autostart"
            checked={settings.autostart}
            onCheckedChange={(autostart) => {
              patch({ autostart });
            }}
          />
        </div>
        <div className="flex items-center justify-between gap-3">
          <span id="ob-minimized" className="text-body text-fg">
            {t("done.startMinimized")}
          </span>
          <Toggle
            aria-labelledby="ob-minimized"
            checked={settings.start_minimized}
            onCheckedChange={(start_minimized) => {
              patch({ start_minimized });
            }}
          />
        </div>
      </div>

      <section aria-label={t("done.tipsTitle")} className="flex w-full max-w-sm flex-col gap-1">
        <h3 className="text-caption font-medium text-fg-2">{t("done.tipsTitle")}</h3>
        <ul className="flex list-disc flex-col gap-1 pl-4 text-caption text-fg-3">
          <li>{t("done.tipWifi")}</li>
          <li>{t("done.tipTray")}</li>
          <li>{t("done.tipShortcuts")}</li>
        </ul>
      </section>
    </div>
  );
}
