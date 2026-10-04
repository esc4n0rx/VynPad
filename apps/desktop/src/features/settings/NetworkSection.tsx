import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { messageForError } from "@/i18n/errors";
import type { Settings } from "@/lib/api/settings";
import { useNetworkInterfaces, useServerInfo } from "@/lib/api/server";
import {
  isCancelled,
  useFirewallStatus,
  useFixFirewall,
  useOpenNetworkSettings,
  useRestartApp,
  useTestConnection,
} from "@/lib/api/system";
import type { Ports } from "@/lib/bindings";
import { useToasts } from "@/lib/toast";
import { portProblem } from "./portRules";
import { SettingRow } from "./SettingRow";

interface SectionProps {
  settings: Settings;
  patch: (change: Partial<Settings>) => void;
}

function PortsRow({ settings, patch }: SectionProps) {
  const { t } = useTranslation("settings");
  const server = useServerInfo();
  const restart = useRestartApp();
  const [draft, setDraft] = useState<Record<keyof Ports, string>>({
    control: String(settings.ports.control),
    input: String(settings.ports.input),
    discovery: String(settings.ports.discovery),
  });
  const parsed: Ports = {
    control: Number(draft.control),
    input: Number(draft.input),
    discovery: Number(draft.discovery),
  };
  const problem = portProblem(parsed);
  const changed =
    parsed.control !== settings.ports.control ||
    parsed.input !== settings.ports.input ||
    parsed.discovery !== settings.ports.discovery;

  const field = (key: keyof Ports) => (
    <label key={key} className="flex flex-col gap-1 text-caption text-fg-2">
      {t(`network.ports.${key}`)}
      <Input
        inputMode="numeric"
        value={draft[key]}
        aria-invalid={problem !== null}
        onChange={(e) => {
          setDraft((d) => ({ ...d, [key]: e.target.value.replace(/\D/g, "").slice(0, 5) }));
        }}
        className="w-28 tabular-nums"
      />
    </label>
  );

  return (
    <SettingRow id="ports" stacked>
      {() => (
        <div className="flex flex-col gap-3">
          <div className="flex flex-wrap items-end gap-3">
            {(["control", "input", "discovery"] as const).map(field)}
            <Button
              variant="primary"
              disabled={!changed || problem !== null}
              onClick={() => {
                patch({ ports: parsed });
              }}
            >
              {t("network.ports.apply")}
            </Button>
          </div>
          {problem && (
            <p role="alert" className="text-caption text-danger">
              {t(`network.ports.${problem}`)}
            </p>
          )}
          {server.data?.ports_pending_restart && (
            <div className="flex flex-wrap items-center gap-3">
              <Badge status="warning">{t("network.ports.pending")}</Badge>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  restart.mutate();
                }}
              >
                {t("network.ports.restart")}
              </Button>
            </div>
          )}
        </div>
      )}
    </SettingRow>
  );
}

function CopyIp({ ip }: { ip: string }) {
  const { t } = useTranslation("settings");
  const [copied, setCopied] = useState(false);
  return (
    <Button
      variant="ghost"
      size="sm"
      aria-label={t("network.interfaces.copy", { ip })}
      onClick={() => {
        void navigator.clipboard.writeText(ip).then(() => {
          setCopied(true);
          window.setTimeout(() => {
            setCopied(false);
          }, 1500);
        });
      }}
    >
      {copied ? <Check /> : <Copy />}
      {copied ? t("network.interfaces.copied") : null}
    </Button>
  );
}

function InterfacesRow() {
  const { t } = useTranslation("settings");
  const interfaces = useNetworkInterfaces();
  const list = interfaces.data ?? [];

  return (
    <SettingRow id="interfaces" stacked>
      {() =>
        list.length === 0 ? (
          <p className="text-body text-fg-2">{t("network.interfaces.empty")}</p>
        ) : (
          <ul className="flex flex-col divide-y divide-separator rounded-lg bg-control">
            {list.map((nic) => (
              <li key={`${nic.name}-${nic.ip}`} className="flex items-center gap-3 px-3 py-2">
                <span className="flex-1 truncate text-body text-fg">{nic.name}</span>
                {nic.auto && <Badge status="success">{t("network.interfaces.auto")}</Badge>}
                {nic.virtual_like && <Badge>{t("network.interfaces.virtual")}</Badge>}
                <span className="text-body text-fg-2 tabular-nums select-text">{nic.ip}</span>
                <CopyIp ip={nic.ip} />
              </li>
            ))}
          </ul>
        )
      }
    </SettingRow>
  );
}

function TestRow() {
  const { t } = useTranslation("settings");
  const test = useTestConnection();
  return (
    <SettingRow id="testConnection" stacked>
      {() => (
        <div className="flex flex-col gap-2">
          <div>
            <Button
              variant="secondary"
              disabled={test.isPending}
              onClick={() => {
                test.mutate();
              }}
            >
              {test.isPending ? t("network.test.running") : t("network.test.run")}
            </Button>
          </div>
          {test.data && (
            <ul className="flex flex-col gap-1">
              {test.data.map((probe) => (
                <li key={probe.what} className="flex items-center gap-2 text-body text-fg">
                  <Badge status={probe.ok ? "success" : "danger"}>
                    {probe.ok ? t("network.test.ok") : t("network.test.fail")}
                  </Badge>
                  {probe.what}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </SettingRow>
  );
}

function FirewallRow() {
  const { t } = useTranslation("settings");
  const status = useFirewallStatus();
  const fix = useFixFirewall();
  const openNetwork = useOpenNetworkSettings();
  const data = status.data;

  const summary = !data
    ? ""
    : !data.applicable
      ? t("network.firewall.notApplicable")
      : !data.firewall_on
        ? t("network.firewall.off")
        : data.ok
          ? t("network.firewall.ok")
          : t("network.firewall.blocked");

  return (
    <SettingRow id="firewall" stacked>
      {() => (
        <div className="flex flex-col gap-3">
          {data && (
            <div className="flex flex-wrap items-center gap-3">
              <Badge status={data.ok ? "success" : "warning"}>{summary}</Badge>
              {data.applicable && !data.ok && (
                <Button
                  variant="primary"
                  size="sm"
                  disabled={fix.isPending}
                  onClick={() => {
                    fix.mutate(undefined, {
                      onSuccess: () => {
                        useToasts.getState().push("success", t("network.firewall.fixed"));
                      },
                      onError: (error) => {
                        useToasts
                          .getState()
                          .push(
                            "error",
                            isCancelled(error)
                              ? t("network.firewall.cancelled")
                              : messageForError(error),
                          );
                      },
                    });
                  }}
                >
                  {fix.isPending ? t("network.firewall.fixing") : t("network.firewall.fix")}
                </Button>
              )}
            </div>
          )}
          {data?.applicable && data.firewall_on && (
            <ul className="flex flex-wrap gap-2">
              {data.ports.map((port) => (
                <li key={`${port.protocol}-${String(port.port)}`}>
                  <Badge status={port.allowed ? "success" : "danger"} title={port.purpose}>
                    {t("network.firewall.port", {
                      protocol: port.protocol.toUpperCase(),
                      port: port.port,
                    })}
                    {" · "}
                    {port.allowed
                      ? t("network.firewall.allowed")
                      : t("network.firewall.blockedPort")}
                  </Badge>
                </li>
              ))}
            </ul>
          )}
          {data?.public_network && (
            <div className="flex flex-wrap items-center gap-3">
              <Badge status="warning">{t("network.firewall.public")}</Badge>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  openNetwork.mutate();
                }}
              >
                {t("network.firewall.openNetwork")}
              </Button>
            </div>
          )}
        </div>
      )}
    </SettingRow>
  );
}

/** Rede: portas, interfaces, teste de conexão e firewall. */
export function NetworkSection({ settings, patch }: SectionProps) {
  return (
    <>
      <PortsRow
        key={`${String(settings.ports.control)}-${String(settings.ports.input)}-${String(settings.ports.discovery)}`}
        settings={settings}
        patch={patch}
      />
      <InterfacesRow />
      <TestRow />
      <FirewallRow />
    </>
  );
}
