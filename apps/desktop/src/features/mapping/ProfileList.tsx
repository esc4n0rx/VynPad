import { useRef, useState, type ChangeEvent } from "react";
import { Copy, Download, MoreHorizontal, Plus, RotateCcw, Trash2, Upload } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ActionMenu, type ActionMenuItem } from "@/components/ui/action-menu";
import { Badge } from "@/components/ui/badge";
import { Button, IconButton } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import {
  useDeleteProfile,
  useDuplicateProfile,
  useExportProfile,
  useImportProfile,
  useRestoreDefaultProfile,
  useSaveProfile,
} from "@/lib/api/profiles";
import { isAppError } from "@/lib/errors";
import type { ProfileSummaryDto } from "@/lib/bindings";
import { notifyError, useToasts } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { ProfileIcon } from "./ProfileIcon";

interface ProfileListProps {
  profiles: readonly ProfileSummaryDto[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

/** Salva o texto num arquivo `.padprofile` (o `<a download>` abre o "Salvar como" da webview). */
function download(name: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = `${name.replace(/[\\/:*?"<>|]/g, "_")}.padprofile`;
  link.click();
  window.setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 1000);
}

/** Coluna da esquerda: os perfis, com criar, duplicar, importar, exportar e excluir. */
export function ProfileList({ profiles, selectedId, onSelect }: ProfileListProps) {
  const { t } = useTranslation("mapping");
  const save = useSaveProfile();
  const duplicate = useDuplicateProfile();
  const importProfile = useImportProfile();
  const exportProfile = useExportProfile();
  const restore = useRestoreDefaultProfile();
  const remove = useDeleteProfile();
  const fileInput = useRef<HTMLInputElement>(null);
  const [toDelete, setToDelete] = useState<{
    profile: ProfileSummaryDto;
    inUse: { devices: number; slots: number } | null;
  } | null>(null);
  const [confirmRestore, setConfirmRestore] = useState(false);

  const create = () => {
    save.mutate(
      {
        id: null,
        input: { name: t("profiles.newName"), description: null, icon: null, profile: {} },
      },
      {
        onSuccess: (created) => {
          onSelect(created.summary.id);
        },
      },
    );
  };

  const onFile = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    void file.text().then((text) => {
      importProfile.mutate(text, {
        onSuccess: (imported) => {
          onSelect(imported.summary.id);
          useToasts
            .getState()
            .push("success", t("profiles.imported", { name: imported.summary.name }));
        },
      });
    });
  };

  const askDelete = (profile: ProfileSummaryDto) => {
    // Primeiro sem confirmar: se estiver em uso, o backend responde com os números.
    remove.mutate(
      { id: profile.id, confirm: false },
      {
        onError: (error) => {
          if (isAppError(error) && error.code === "profile_in_use") {
            setToDelete({ profile, inUse: error.in_use ?? { devices: 0, slots: 0 } });
          } else {
            notifyError(error);
          }
        },
        onSuccess: () => {
          if (selectedId === profile.id) onSelect(profiles[0]?.id ?? "");
        },
      },
    );
  };

  const menuFor = (profile: ProfileSummaryDto): ActionMenuItem[] => [
    {
      key: "duplicate",
      label: t("profiles.duplicate"),
      icon: <Copy />,
      onSelect: () => {
        duplicate.mutate(
          { id: profile.id, name: `${profile.name} (${t("profiles.copySuffix")})` },
          {
            onSuccess: (copy) => {
              onSelect(copy.summary.id);
            },
          },
        );
      },
    },
    {
      key: "export",
      label: t("profiles.export"),
      icon: <Download />,
      onSelect: () => {
        exportProfile.mutate(profile.id, {
          onSuccess: (text) => {
            download(profile.name, text);
          },
        });
      },
    },
    {
      key: "delete",
      label: t("profiles.delete"),
      icon: <Trash2 />,
      destructive: true,
      separated: true,
      disabled: profile.is_builtin,
      onSelect: () => {
        askDelete(profile);
      },
    },
  ];

  return (
    <aside aria-label={t("profiles.list")} className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex items-center gap-2">
        <h2 className="flex-1 text-subtitle font-semibold text-fg">{t("profiles.title")}</h2>
        <IconButton
          label={t("profiles.import")}
          size="sm"
          onClick={() => fileInput.current?.click()}
        >
          <Upload />
        </IconButton>
        <IconButton label={t("profiles.new")} size="sm" variant="secondary" onClick={create}>
          <Plus />
        </IconButton>
        <input
          ref={fileInput}
          type="file"
          accept=".padprofile,application/json"
          aria-label={t("profiles.import")}
          className="hidden"
          onChange={onFile}
        />
      </div>

      {profiles.length === 0 ? (
        <p className="text-body text-fg-2">{t("profiles.none")}</p>
      ) : (
        <ul className="flex min-h-0 flex-1 flex-col gap-1 overflow-auto">
          {profiles.map((profile) => (
            <li key={profile.id} className="flex items-center gap-1">
              <button
                type="button"
                aria-current={profile.id === selectedId ? "true" : undefined}
                onClick={() => {
                  onSelect(profile.id);
                }}
                className={cn(
                  "flex min-w-0 flex-1 items-center gap-2 rounded-md px-2.5 py-2 text-left text-body focus-ring transition-colors",
                  profile.id === selectedId
                    ? "bg-accent text-on-accent"
                    : "text-fg hover:bg-control",
                )}
              >
                <ProfileIcon name={profile.icon} className="size-4 shrink-0" />
                <span className="flex-1 truncate">{profile.name}</span>
                {profile.is_builtin && <Badge className="shrink-0">{t("profiles.builtin")}</Badge>}
              </button>
              <ActionMenu
                trigger={
                  <IconButton label={t("profiles.menu", { name: profile.name })} size="sm">
                    <MoreHorizontal />
                  </IconButton>
                }
                items={menuFor(profile)}
              />
            </li>
          ))}
        </ul>
      )}

      <Button
        variant="ghost"
        size="sm"
        onClick={() => {
          setConfirmRestore(true);
        }}
      >
        <RotateCcw />
        {t("profiles.restore")}
      </Button>

      <ConfirmDialog
        open={toDelete !== null}
        onOpenChange={(open) => {
          if (!open) setToDelete(null);
        }}
        title={t("confirmDelete.title", { name: toDelete?.profile.name ?? "" })}
        description={
          toDelete?.inUse ? t("confirmDelete.bodyInUse", toDelete.inUse) : t("confirmDelete.body")
        }
        confirmLabel={t("confirmDelete.delete")}
        cancelLabel={t("confirmDelete.cancel")}
        destructive
        onConfirm={() => {
          if (!toDelete) return;
          const { profile } = toDelete;
          remove.mutate(
            { id: profile.id, confirm: true },
            {
              onSuccess: () => {
                if (selectedId === profile.id) onSelect(profiles[0]?.id ?? "");
              },
            },
          );
        }}
      />
      <ConfirmDialog
        open={confirmRestore}
        onOpenChange={setConfirmRestore}
        title={t("confirmRestore.title")}
        description={t("confirmRestore.body")}
        confirmLabel={t("confirmRestore.restore")}
        cancelLabel={t("confirmDelete.cancel")}
        onConfirm={() => {
          restore.mutate(undefined, {
            onSuccess: () => {
              useToasts.getState().push("success", t("profiles.restored"));
            },
          });
        }}
      />
    </aside>
  );
}
