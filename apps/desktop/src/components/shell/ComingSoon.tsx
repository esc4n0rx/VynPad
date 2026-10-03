import type { LucideIcon } from "lucide-react";
import { EmptyState } from "@/components/ui/empty-state";

/** Tela ainda sem conteúdo (F6); marca o lugar e diz qual task a preenche. */
export function ComingSoon({
  icon: Icon,
  title,
  task,
}: {
  icon: LucideIcon;
  title: string;
  task: string;
}) {
  return (
    <div className="flex h-full items-center justify-center">
      <EmptyState
        icon={<Icon strokeWidth={1.75} />}
        title={title}
        description={`Esta tela chega na ${task}.`}
      />
    </div>
  );
}
