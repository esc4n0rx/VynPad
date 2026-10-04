import { useMemo, useState, type ReactNode } from "react";
import { Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettings, useSettingsPatch, type Settings } from "@/lib/api/settings";
import { cn } from "@/lib/utils";
import { AppearanceSection } from "./AppearanceSection";
import { ControlsSection } from "./ControlsSection";
import { GeneralSection } from "./GeneralSection";
import { NetworkSection } from "./NetworkSection";
import { AboutSection, LanguageSection, PrivacySection } from "./PrivacyAndAboutSections";
import {
  ITEMS,
  SECTIONS,
  itemHintKey,
  itemLabelKey,
  matchingItems,
  type SectionId,
} from "./settingsIndex";
import { VisibleItems } from "./visibleItems";

interface SectionProps {
  settings: Settings;
  patch: (change: Partial<Settings>) => void;
}

function renderSection(id: SectionId, props: SectionProps): ReactNode {
  switch (id) {
    case "general":
      return <GeneralSection {...props} />;
    case "network":
      return <NetworkSection {...props} />;
    case "controls":
      return <ControlsSection {...props} />;
    case "appearance":
      return <AppearanceSection {...props} />;
    case "language":
      return <LanguageSection {...props} />;
    case "privacy":
      return <PrivacySection />;
    case "about":
      return <AboutSection />;
  }
}

/** Configurações no estilo "Ajustes do Sistema": seções à esquerda, busca no topo. */
export function SettingsPage() {
  const { t } = useTranslation("settings");
  const { data: settings } = useSettings();
  const { patch } = useSettingsPatch();
  const [active, setActive] = useState<SectionId>("general");
  const [query, setQuery] = useState("");

  const visible = useMemo(
    () =>
      matchingItems(query, (id) =>
        [
          t(itemLabelKey(id)),
          t(itemHintKey(id)),
          t(`sections.${ITEMS.find((i) => i.id === id)?.section ?? "general"}`),
        ].join(" "),
      ),
    // `t` muda quando o idioma muda.
    [query, t],
  );
  const searching = query.trim() !== "";
  const shown = SECTIONS.filter((section) =>
    searching ? ITEMS.some((i) => i.section === section && visible.has(i.id)) : section === active,
  );

  return (
    <div className="flex h-full min-h-0 flex-col gap-4 p-6">
      <header className="flex flex-wrap items-center gap-3">
        <h1 className="flex-1 text-title font-semibold tracking-tight">{t("title")}</h1>
        <div className="relative w-64">
          <Search
            aria-hidden
            className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-fg-3"
          />
          <Input
            type="search"
            aria-label={t("search.label")}
            placeholder={t("search.placeholder")}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
            }}
            className="pl-8"
          />
        </div>
      </header>

      <div className="flex min-h-0 flex-1 gap-6">
        <nav aria-label={t("nav")} className="flex w-48 shrink-0 flex-col gap-0.5">
          {SECTIONS.map((section) => (
            <button
              key={section}
              type="button"
              aria-current={!searching && section === active ? "page" : undefined}
              onClick={() => {
                setQuery("");
                setActive(section);
              }}
              className={cn(
                "rounded-md px-3 py-2 text-left text-body focus-ring transition-colors",
                !searching && section === active
                  ? "bg-accent text-on-accent"
                  : "text-fg hover:bg-control",
              )}
            >
              {t(`sections.${section}`)}
            </button>
          ))}
        </nav>

        <div className="min-w-0 flex-1 overflow-auto pr-2">
          {!settings ? (
            <div className="flex flex-col gap-3">
              {[0, 1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-14 rounded-lg" />
              ))}
            </div>
          ) : shown.length === 0 ? (
            <p role="status" className="text-body text-fg-2">
              {t("search.empty", { query })}
            </p>
          ) : (
            <VisibleItems.Provider value={visible}>
              {shown.map((section) => (
                <section
                  key={section}
                  aria-labelledby={`section-${section}`}
                  className="mb-6 flex flex-col"
                >
                  <h2
                    id={`section-${section}`}
                    className="mb-1 text-subtitle font-semibold text-fg"
                  >
                    {t(`sections.${section}`)}
                  </h2>
                  <div className="flex flex-col divide-y divide-separator">
                    {renderSection(section, { settings, patch })}
                  </div>
                </section>
              ))}
            </VisibleItems.Provider>
          )}
        </div>
      </div>
    </div>
  );
}
