import { Gamepad2 } from "lucide-react";
import { PROFILE_ICONS, isKnownIcon } from "./profileIcons";

/** O ícone do perfil; um nome desconhecido (ou ausente) mostra o gamepad. */
export function ProfileIcon({ name, className }: { name: string | null; className?: string }) {
  const Icon = isKnownIcon(name) ? PROFILE_ICONS[name] : Gamepad2;
  return <Icon aria-hidden className={className} strokeWidth={1.75} />;
}
