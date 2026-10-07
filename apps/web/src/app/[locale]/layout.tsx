import "@fontsource-variable/inter";
import type { Metadata, Viewport } from "next";
import { notFound } from "next/navigation";
import type { ReactNode } from "react";
import { Footer } from "@/components/footer";
import { Header } from "@/components/header";
import { Providers } from "@/components/providers";
import { ThemeScript } from "@/components/theme-script";
import { readConfig } from "@/lib/config";
import { isLocale, LOCALES, type Locale } from "@/i18n/config";
import { getDictionary, intlLocale } from "@/i18n";
import "../globals.css";

export const dynamicParams = false;

export function generateStaticParams() {
  return LOCALES.map((locale) => ({ locale }));
}

/* eslint-disable no-restricted-syntax -- `theme-color` é uma meta tag lida pelo navegador, fora do CSS: precisa do valor literal (o mesmo `--bg-page` de cada tema). */
export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  colorScheme: "dark light",
  themeColor: [
    { media: "(prefers-color-scheme: dark)", color: "#1c1c1e" },
    { media: "(prefers-color-scheme: light)", color: "#f5f5f7" },
  ],
};
/* eslint-enable no-restricted-syntax */

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  if (!isLocale(locale)) return {};
  const dict = getDictionary(locale);
  const { siteUrl } = readConfig();
  return {
    metadataBase: new URL(siteUrl),
    title: { default: dict.meta.title, template: "%s · VynPad" },
    description: dict.meta.description,
    applicationName: "VynPad",
    alternates: {
      canonical: `/${locale}`,
      languages: { "pt-BR": "/pt-BR", en: "/en", "x-default": "/pt-BR" },
    },
    openGraph: {
      type: "website",
      siteName: "VynPad",
      title: dict.meta.title,
      description: dict.meta.description,
      locale: intlLocale(locale).replace("-", "_"),
      url: `/${locale}`,
    },
    twitter: {
      card: "summary_large_image",
      title: dict.meta.title,
      description: dict.meta.description,
    },
    robots: { index: true, follow: true },
  };
}

export default async function LocaleLayout({
  children,
  params,
}: {
  children: ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale: raw } = await params;
  if (!isLocale(raw)) notFound();
  const locale: Locale = raw;
  const dict = getDictionary(locale);
  const { githubUrl } = readConfig();

  return (
    <html lang={locale} suppressHydrationWarning data-theme="dark">
      <head>
        <ThemeScript />
        {/* Sem JavaScript as seções que entram animadas continuam visíveis. */}
        <noscript>
          <style>{`.reveal{opacity:1!important;transform:none!important}`}</style>
        </noscript>
      </head>
      <body>
        <a
          href="#content"
          className="fixed top-3 left-3 z-[100] -translate-y-20 rounded-full bg-accent px-4 py-2 text-sm font-semibold text-on-accent focus-ring focus:translate-y-0"
        >
          {dict.a11y.skip}
        </a>
        <Providers>
          {/* Só as fatias que o navegador usa: o dicionário inteiro não precisa descer. */}
          <Header
            locale={locale}
            dict={{ nav: dict.nav, theme: dict.theme, locale: dict.locale }}
          />
          <main id="content">{children}</main>
          <Footer locale={locale} dict={dict} githubUrl={githubUrl} />
        </Providers>
      </body>
    </html>
  );
}
