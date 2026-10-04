import { useContext, useId, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { itemHintKey, itemLabelKey, type ItemId } from "./settingsIndex";
import { VisibleItems } from "./visibleItems";

interface SettingRowProps {
  id: ItemId;
  /** O controle; recebe o `id` do rótulo e o da dica (para `aria-labelledby` e `aria-describedby`). */
  children: (ids: { labelId: string; hintId: string }) => ReactNode;
  /** Controles largos (listas, grades) ocupam a linha toda, abaixo do texto. */
  stacked?: boolean;
  /** Sem a dica (quando ela já está dentro do controle). */
  hideHint?: boolean;
}

/** Uma linha de configuração: rótulo e dica à esquerda, controle à direita. */
export function SettingRow({ id, children, stacked, hideHint }: SettingRowProps) {
  const { t } = useTranslation("settings");
  const visible = useContext(VisibleItems);
  const uid = useId();
  if (visible && !visible.has(id)) return null;
  const labelId = `${uid}-label`;
  const hintId = `${uid}-hint`;

  return (
    <div
      data-setting={id}
      className={cn(
        "flex gap-4 py-3",
        stacked ? "flex-col" : "flex-wrap items-center justify-between",
      )}
    >
      <div className="max-w-xl min-w-0 flex-1 basis-64">
        <p id={labelId} className="text-body font-medium text-fg">
          {t(itemLabelKey(id))}
        </p>
        {!hideHint && (
          <p id={hintId} className="text-caption text-fg-2">
            {t(itemHintKey(id))}
          </p>
        )}
      </div>
      <div className={cn("min-w-0", stacked ? "w-full" : "shrink-0")}>
        {children({ labelId, hintId })}
      </div>
    </div>
  );
}
