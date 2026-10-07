import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Container } from "@/components/ui";
import { getDictionary, intlLocale } from "@/i18n";
import { fmt, isLocale } from "@/i18n/config";
import { readConfig } from "@/lib/config";
import { getReleases } from "@/lib/github";
import { renderMarkdown } from "@/lib/markdown";

export const revalidate = 300;

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  if (!isLocale(locale)) return {};
  return {
    title: getDictionary(locale).changelog.title,
    alternates: {
      canonical: `/${locale}/changelog`,
      languages: { "pt-BR": "/pt-BR/changelog", en: "/en/changelog" },
    },
  };
}

export default async function ChangelogPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  if (!isLocale(locale)) notFound();
  const { changelog } = getDictionary(locale);
  const config = readConfig();
  const releases = await getReleases(20, config);
  const dateFormat = new Intl.DateTimeFormat(intlLocale(locale), {
    day: "2-digit",
    month: "long",
    year: "numeric",
  });

  return (
    <Container className="max-w-3xl py-16 sm:py-24">
      <h1 className="text-h2 font-semibold text-balance">{changelog.title}</h1>
      <p className="mt-4 text-lead text-fg-2">{changelog.lead}</p>

      {releases.length === 0 ? (
        <p className="mt-12 rounded-3xl p-6 text-fg-2 glass hairline">{changelog.empty}</p>
      ) : (
        <ol className="mt-12 space-y-6">
          {releases.map((release) => {
            const date = release.publishedAt ? new Date(release.publishedAt) : undefined;
            return (
              <li key={release.tag}>
                <article className="rounded-3xl p-6 glass hairline sm:p-8">
                  <header className="flex flex-wrap items-baseline gap-x-4 gap-y-1">
                    <h2 className="text-2xl font-semibold tracking-tight">{release.version}</h2>
                    {release.prerelease && (
                      <span className="rounded-full bg-control px-2.5 py-0.5 text-xs font-semibold text-warning">
                        {changelog.prerelease}
                      </span>
                    )}
                    {date && !Number.isNaN(date.getTime()) && (
                      <time dateTime={date.toISOString()} className="text-sm text-fg-3">
                        {fmt(changelog.published, { date: dateFormat.format(date) })}
                      </time>
                    )}
                  </header>
                  <div className="mt-4 text-[15px] leading-relaxed text-fg-2">
                    {release.notes.trim() ? (
                      renderMarkdown(release.notes)
                    ) : (
                      <p>{changelog.noNotes}</p>
                    )}
                  </div>
                  {/* O repositório pode ser privado: o link só aparece se o endereço público foi informado. */}
                  {config.githubUrl && (
                    <a
                      href={`${config.githubUrl.replace(/\/$/, "")}/releases/tag/${encodeURIComponent(release.tag)}`}
                      rel="noopener noreferrer"
                      className="mt-5 inline-block rounded text-sm font-medium text-accent focus-ring hover:underline"
                    >
                      {changelog.github}
                    </a>
                  )}
                </article>
              </li>
            );
          })}
        </ol>
      )}
    </Container>
  );
}
