import type { ReactElement, ReactNode } from "react";
import { ChevronRight } from "lucide-react";
import { DropdownMenu } from "radix-ui";
import { cn } from "@/lib/utils";

export interface ActionMenuItem {
  key: string;
  label: ReactNode;
  icon?: ReactNode;
  onSelect?: () => void;
  destructive?: boolean;
  disabled?: boolean;
  /** Subitens: o item vira um submenu e `onSelect` é ignorado. */
  items?: readonly ActionMenuItem[];
  /** Linha separadora antes do item. */
  separated?: boolean;
}

interface ActionMenuProps {
  /** O gatilho (precisa aceitar `ref`), em geral um `IconButton`. */
  trigger: ReactElement;
  items: readonly ActionMenuItem[];
  align?: "start" | "center" | "end";
}

const ITEM =
  "relative flex h-7 cursor-default items-center gap-2 rounded-sm px-2 text-body outline-none select-none data-[disabled]:opacity-40 data-[highlighted]:bg-accent data-[highlighted]:text-on-accent [&_svg]:size-4 [&_svg]:shrink-0";
const CONTENT = "z-50 min-w-44 rounded-lg p-1 shadow-popover glass";

function Entries({ items }: { items: readonly ActionMenuItem[] }) {
  return items.map((item) => (
    <div key={item.key}>
      {item.separated && <DropdownMenu.Separator className="my-1 h-px bg-separator" />}
      {item.items ? (
        <DropdownMenu.Sub>
          <DropdownMenu.SubTrigger disabled={item.disabled} className={cn(ITEM, "text-fg")}>
            {item.icon}
            <span className="flex-1">{item.label}</span>
            <ChevronRight aria-hidden className="size-3.5" />
          </DropdownMenu.SubTrigger>
          <DropdownMenu.Portal>
            <DropdownMenu.SubContent sideOffset={4} className={CONTENT}>
              <Entries items={item.items} />
            </DropdownMenu.SubContent>
          </DropdownMenu.Portal>
        </DropdownMenu.Sub>
      ) : (
        <DropdownMenu.Item
          disabled={item.disabled}
          onSelect={item.onSelect}
          className={cn(ITEM, item.destructive ? "text-danger" : "text-fg")}
        >
          {item.icon}
          {item.label}
        </DropdownMenu.Item>
      )}
    </div>
  ));
}

/** Menu de ações (o botão `…` dos cards), com submenus e teclado (setas, Enter, Esc). */
export function ActionMenu({ trigger, items, align = "end" }: ActionMenuProps) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>{trigger}</DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content align={align} sideOffset={6} className={CONTENT}>
          <Entries items={items} />
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
