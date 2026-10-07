"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { LOCALE_COOKIE, LOCALES, type Locale } from "@/i18n/config";
import { cn } from "@/lib/cn";

interface LocaleSwitchProps {
  current: Locale;
  labels: { label: string; pt: string; en: string };
}

const SHORT: Record<Locale, string> = { "pt-BR": "PT", en: "EN" };

/** Troca o idioma mantendo a página (e a âncora) e lembra a escolha por um ano. */
export function LocaleSwitch({ current, labels }: LocaleSwitchProps) {
  const pathname = usePathname();
  const rest = pathname.replace(/^\/(pt-BR|en)(?=\/|$)/, "");

  return (
    <nav
      aria-label={labels.label}
      className="inline-flex h-9 items-center rounded-full bg-control p-0.5"
    >
      {LOCALES.map((locale) => {
        const active = locale === current;
        const name = locale === "pt-BR" ? labels.pt : labels.en;
        return (
          <Link
            key={locale}
            href={`/${locale}${rest}`}
            hrefLang={locale}
            lang={locale}
            aria-current={active ? "true" : undefined}
            aria-label={name}
            title={name}
            onClick={() => {
              document.cookie = `${LOCALE_COOKIE}=${locale}; path=/; max-age=31536000; samesite=lax`;
            }}
            className={cn(
              "grid h-8 min-w-9 place-items-center rounded-full px-2 text-xs font-semibold tracking-wide focus-ring transition-colors",
              active ? "bg-page text-fg shadow-sm" : "text-fg-3 hover:text-fg-2",
            )}
          >
            {SHORT[locale]}
          </Link>
        );
      })}
    </nav>
  );
}
