import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Check, MoreHorizontal, Pencil, ShieldOff, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { DeviceIcon, type DeviceStatus } from "@/components/icons/DeviceIcon";
import { ActionMenu, type ActionMenuItem } from "@/components/ui/action-menu";
import { Badge } from "@/components/ui/badge";
import { IconButton } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Input } from "@/components/ui/input";
import { formatDateTime } from "@/i18n/format";
import {
  useForgetDevice,
  useRenameDevice,
  useRevokeDevice,
  useSetDeviceDefaults,
} from "@/lib/api/devices";
import { useProfiles } from "@/lib/api/profiles";
import type { DeviceDto } from "@/lib/bindings";

function statusOf(device: DeviceDto): DeviceStatus {
  if (device.revoked) return "offline";
  return device.current_slot === null ? "idle" : "connected";
}

function Row({ device }: { device: DeviceDto }) {
  const { t } = useTranslation("devices");
  const profiles = useProfiles();
  const rename = useRenameDevice();
  const revoke = useRevokeDevice();
  const forget = useForgetDevice();
  const defaults = useSetDeviceDefaults();
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(device.name);
  const [confirm, setConfirm] = useState<"revoke" | "forget" | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Ao entrar no modo de edição, o nome já vem selecionado.
  useEffect(() => {
    if (editing) inputRef.current?.select();
  }, [editing]);

  const startEditing = () => {
    setName(device.name);
    setEditing(true);
  };
  const save = () => {
    const next = name.trim();
    setEditing(false);
    if (next !== "" && next !== device.name) rename.mutate({ id: device.id, name: next });
  };
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Enter") save();
    if (event.key === "Escape") setEditing(false);
  };

  const setDefaults = (
    patch: Partial<{ preferred_slot: number | null; default_profile_id: string | null }>,
  ) => {
    defaults.mutate({
      id: device.id,
      defaults: {
        preferred_slot: device.preferred_slot,
        default_profile_id: device.default_profile_id,
        color: device.color,
        ...patch,
      },
    });
  };

  const mark = (on: boolean) => (on ? <Check /> : <span className="size-4" />);
  const menu: ActionMenuItem[] = [
    { key: "rename", label: t("menu.rename"), icon: <Pencil />, onSelect: startEditing },
    {
      key: "slot",
      label: t("menu.setSlot"),
      items: [
        {
          key: "slot-auto",
          label: t("menu.slotAuto"),
          icon: mark(device.preferred_slot === null),
          onSelect: () => {
            setDefaults({ preferred_slot: null });
          },
        },
        ...[0, 1, 2, 3].map((slot) => ({
          key: `slot-${String(slot)}`,
          label: `P${String(slot + 1)}`,
          icon: mark(device.preferred_slot === slot),
          onSelect: () => {
            setDefaults({ preferred_slot: slot });
          },
        })),
      ],
    },
    {
      key: "profile",
      label: t("menu.setProfile"),
      items: [
        {
          key: "profile-app",
          label: t("menu.profileApp"),
          icon: mark(device.default_profile_id === null),
          onSelect: () => {
            setDefaults({ default_profile_id: null });
          },
        },
        ...(profiles.data ?? []).map((profile) => ({
          key: `profile-${profile.id}`,
          label: profile.name,
          icon: mark(device.default_profile_id === profile.id),
          onSelect: () => {
            setDefaults({ default_profile_id: profile.id });
          },
        })),
      ],
    },
    {
      key: "revoke",
      label: t("menu.revoke"),
      icon: <ShieldOff />,
      destructive: true,
      disabled: device.revoked,
      separated: true,
      onSelect: () => {
        setConfirm("revoke");
      },
    },
    {
      key: "forget",
      label: t("menu.forget"),
      icon: <Trash2 />,
      destructive: true,
      onSelect: () => {
        setConfirm("forget");
      },
    },
  ];

  const profileName = profiles.data?.find((p) => p.id === device.default_profile_id)?.name;

  return (
    <Card className="flex items-center gap-3">
      <DeviceIcon status={statusOf(device)} />
      <div className="min-w-0 flex-1">
        {editing ? (
          <Input
            ref={inputRef}
            aria-label={t("rename.label")}
            value={name}
            onChange={(e) => {
              setName(e.target.value);
            }}
            onKeyDown={onKey}
            onBlur={save}
            maxLength={64}
            className="h-7"
          />
        ) : (
          <p className="flex items-center gap-2 text-body font-medium text-fg">
            <span className="truncate">{device.name}</span>
            {device.revoked && <Badge status="danger">{t("paired.revoked")}</Badge>}
            {device.current_slot !== null && (
              <Badge status="success">
                {t("paired.connectedAt", { slot: device.current_slot + 1 })}
              </Badge>
            )}
          </p>
        )}
        <p className="truncate text-caption text-fg-3">
          {[device.model, device.os_version].filter(Boolean).join(" · ")}
        </p>
        <p className="truncate text-caption text-fg-3">
          {device.last_seen_at === null
            ? t("paired.never")
            : t("paired.lastSeen", { when: formatDateTime(device.last_seen_at) })}
          {" · "}
          {device.preferred_slot === null
            ? t("paired.slotAuto")
            : t("paired.slot", { slot: device.preferred_slot + 1 })}
          {" · "}
          {profileName ? t("paired.profile", { name: profileName }) : t("paired.profileApp")}
        </p>
      </div>
      <ActionMenu
        trigger={
          <IconButton label={t("menu.label", { name: device.name })} size="sm">
            <MoreHorizontal />
          </IconButton>
        }
        items={menu}
      />
      <ConfirmDialog
        open={confirm === "revoke"}
        onOpenChange={(open) => {
          if (!open) setConfirm(null);
        }}
        title={t("confirm.revokeTitle", { name: device.name })}
        description={t("confirm.revokeBody")}
        confirmLabel={t("confirm.revoke")}
        cancelLabel={t("confirm.cancel")}
        destructive
        onConfirm={() => {
          revoke.mutate(device.id);
        }}
      />
      <ConfirmDialog
        open={confirm === "forget"}
        onOpenChange={(open) => {
          if (!open) setConfirm(null);
        }}
        title={t("confirm.forgetTitle", { name: device.name })}
        description={t("confirm.forgetBody")}
        confirmLabel={t("confirm.forget")}
        cancelLabel={t("confirm.cancel")}
        destructive
        onConfirm={() => {
          forget.mutate(device.id);
        }}
      />
    </Card>
  );
}

/** Dispositivos pareados, com nome editável e menu de ações. */
export function PairedSection({ devices }: { devices: readonly DeviceDto[] }) {
  const { t } = useTranslation("devices");
  return (
    <section aria-labelledby="paired-title" className="flex flex-col gap-3">
      <h2 id="paired-title" className="text-subtitle font-semibold text-fg">
        {t("paired.title")}
      </h2>
      <ul className="flex flex-col gap-3">
        {devices.map((device) => (
          <li key={device.id}>
            <Row device={device} />
          </li>
        ))}
      </ul>
    </section>
  );
}
