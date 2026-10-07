import { notFound } from "next/navigation";

// Qualquer endereço desconhecido sob um idioma cai na página 404 (`../not-found.tsx`).
export default function CatchAll(): never {
  notFound();
}
