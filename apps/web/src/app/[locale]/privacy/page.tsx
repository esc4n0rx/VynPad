import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Container } from "@/components/ui";
import { getDictionary, intlLocale } from "@/i18n";
import { fmt, isLocale } from "@/i18n/config";

/** A data em que o texto da política foi revisado pela última vez. */
const UPDATED = "2026-10-11";

/** Contato de suporte e privacidade (o mesmo informado no Google Play Console). */
const SUPPORT_EMAIL = "matheusxpcamper@gmail.com";

/** Troca o `{email}` do texto por um link de e-mail. */
function withEmail(text: string) {
  const [before, after] = text.split("{email}");
  if (after === undefined) return text;
  return (
    <>
      {before}
      <a
        href={`mailto:${SUPPORT_EMAIL}`}
        className="rounded font-medium text-fg underline underline-offset-4 focus-ring"
      >
        {SUPPORT_EMAIL}
      </a>
      {after}
    </>
  );
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  if (!isLocale(locale)) return {};
  return {
    title: getDictionary(locale).privacy.title,
    alternates: {
      canonical: `/${locale}/privacy`,
      languages: { "pt-BR": "/pt-BR/privacy", en: "/en/privacy" },
    },
  };
}

export default async function PrivacyPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  if (!isLocale(locale)) notFound();
  const { privacy } = getDictionary(locale);
  const date = new Intl.DateTimeFormat(intlLocale(locale), {
    day: "2-digit",
    month: "long",
    year: "numeric",
  }).format(new Date(`${UPDATED}T12:00:00Z`));
  return (
    <Container className="max-w-3xl py-16 sm:py-24">
      <article>
        <h1 className="text-h2 font-semibold text-balance">{privacy.title}</h1>
        <p className="mt-3 text-sm text-fg-3">{fmt(privacy.updated, { date })}</p>
        <p className="mt-8 text-lead text-pretty text-fg-2">{privacy.intro}</p>
        <div className="mt-12 space-y-10">
          {privacy.sections.map((section) => (
            <section key={section.title}>
              <h2 className="text-xl font-semibold tracking-tight">{section.title}</h2>
              <p className="mt-3 text-[17px] leading-relaxed text-pretty text-fg-2">
                {withEmail(section.body)}
              </p>
            </section>
          ))}
        </div>
      </article>
    </Container>
  );
}
