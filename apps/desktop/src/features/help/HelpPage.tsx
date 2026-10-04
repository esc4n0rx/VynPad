import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import { Markdown } from "@/components/Markdown";
import { useSettings } from "@/lib/api/settings";
import { HELP_DOCS, docWithHeading, helpSource, type HelpDoc, type HelpLanguage } from "@/lib/help";
import { parseMarkdown } from "@/lib/markdown";
import { cn } from "@/lib/utils";

const isDoc = (value: string | null): value is HelpDoc =>
  value !== null && (HELP_DOCS as readonly string[]).includes(value);

/** A ajuda do app: os documentos de `docs/usuario`, no idioma do app. */
export function HelpPage() {
  const { t } = useTranslation();
  const { data: settings } = useSettings();
  const language: HelpLanguage = settings?.language === "en" ? "en" : "pt-BR";
  const [params, setParams] = useSearchParams();
  const topic = params.get("topic");

  // `?topic=<id>` abre o documento que tem o título; sem ele, vale `?doc=` ou o primeiro.
  const doc: HelpDoc = useMemo(() => {
    if (topic) {
      const found = docWithHeading(topic, language);
      if (found) return found;
    }
    const wanted = params.get("doc");
    return isDoc(wanted) ? wanted : HELP_DOCS[0];
  }, [topic, params, language]);

  const blocks = useMemo(() => parseMarkdown(helpSource(doc, language)), [doc, language]);
  const titles = useMemo(
    () =>
      HELP_DOCS.map((id) => {
        const heading = parseMarkdown(helpSource(id, language)).find(
          (b) => b.type === "heading" && b.level === 1,
        );
        return { id, title: heading?.type === "heading" ? heading.text : id };
      }),
    [language],
  );

  // Chegou por um tópico: rola até o título.
  useEffect(() => {
    if (!topic) return;
    document.getElementById(topic)?.scrollIntoView({ block: "start" });
  }, [topic, doc]);

  const goTo = (id: string) => {
    const target = docWithHeading(id, language);
    if (target) setParams({ doc: target, topic: id });
  };

  return (
    <div className="flex h-full min-h-0 gap-6 p-6">
      <nav aria-label={t("help.nav")} className="flex w-56 shrink-0 flex-col gap-0.5">
        <h1 className="mb-2 text-title font-semibold tracking-tight">{t("help.title")}</h1>
        {titles.map(({ id, title }) => (
          <button
            key={id}
            type="button"
            aria-current={id === doc ? "page" : undefined}
            onClick={() => {
              setParams({ doc: id });
            }}
            className={cn(
              "rounded-md px-3 py-2 text-left text-body focus-ring transition-colors",
              id === doc ? "bg-accent text-on-accent" : "text-fg hover:bg-control",
            )}
          >
            {title}
          </button>
        ))}
      </nav>
      <article className="max-w-3xl min-w-0 flex-1 overflow-auto pr-2">
        <Markdown blocks={blocks} onAnchor={goTo} />
      </article>
    </div>
  );
}
