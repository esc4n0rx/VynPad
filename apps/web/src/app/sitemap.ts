import type { MetadataRoute } from "next";
import { LOCALES } from "@/i18n/config";
import { readConfig } from "@/lib/config";

const PAGES = [
  { path: "", priority: 1, changeFrequency: "weekly" },
  { path: "/changelog", priority: 0.6, changeFrequency: "weekly" },
  { path: "/privacy", priority: 0.3, changeFrequency: "yearly" },
] as const;

export default function sitemap(): MetadataRoute.Sitemap {
  const { siteUrl } = readConfig();
  return LOCALES.flatMap((locale) =>
    PAGES.map((page) => ({
      url: `${siteUrl}/${locale}${page.path}`,
      changeFrequency: page.changeFrequency,
      priority: page.priority,
      // Cada página aponta para as suas versões em todos os idiomas.
      alternates: {
        languages: Object.fromEntries(LOCALES.map((l) => [l, `${siteUrl}/${l}${page.path}`])),
      },
    })),
  );
}
