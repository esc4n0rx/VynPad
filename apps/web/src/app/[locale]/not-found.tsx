"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { buttonClass, Container } from "@/components/ui";
import { en } from "@/i18n/en";
import { ptBR } from "@/i18n/pt-BR";

export default function NotFound() {
  // Esta página não recebe o idioma por parâmetro: ele sai do começo do endereço.
  const english = usePathname().startsWith("/en");
  const { notFound } = english ? en : ptBR;
  return (
    <Container className="grid min-h-[60vh] place-items-center py-24 text-center">
      <div>
        <p className="text-sm font-semibold text-accent tabular-nums">404</p>
        <h1 className="mt-3 text-h2 font-semibold text-balance">{notFound.title}</h1>
        <p className="mt-3 text-lead text-fg-2">{notFound.body}</p>
        <Link href={english ? "/en" : "/pt-BR"} className={buttonClass("primary", "lg", "mt-8")}>
          {notFound.back}
        </Link>
      </div>
    </Container>
  );
}
