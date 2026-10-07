import type { MetadataRoute } from "next";
import { readConfig } from "@/lib/config";

export default function robots(): MetadataRoute.Robots {
  const { siteUrl } = readConfig();
  return {
    // A rota de download redireciona para arquivos: não tem o que indexar.
    rules: { userAgent: "*", allow: "/", disallow: "/api/" },
    sitemap: `${siteUrl}/sitemap.xml`,
  };
}
