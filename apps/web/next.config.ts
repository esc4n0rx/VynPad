import type { NextConfig } from "next";

/**
 * Cabeçalhos de segurança de todas as respostas. O CSP libera só o que o site usa: scripts e
 * estilos próprios (o Next injeta alguns trechos inline, daí `'unsafe-inline'`; não há script
 * de terceiros), imagens locais/`data:` e conexão apenas com a própria origem. O site não carrega
 * nada de CDNs: a fonte vem empacotada.
 */
const dev = process.env.NODE_ENV !== "production";

const csp = [
  "default-src 'self'",
  // O React em desenvolvimento usa `eval` para reconstruir pilhas de erro; em produção não.
  `script-src 'self' 'unsafe-inline'${dev ? " 'unsafe-eval'" : ""}`,
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data: blob:",
  "font-src 'self'",
  "connect-src 'self'",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'self'",
  "frame-ancestors 'none'",
  ...(dev ? [] : ["upgrade-insecure-requests"]),
].join("; ");

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // O `next dev` não deve escrever um AGENTS.md no projeto.
  agentRules: false,
  poweredByHeader: false,
  // As imagens são SVG/PNG pequenos já no tamanho final: sem o otimizador (e sem o `sharp`).
  images: { unoptimized: true },
  async headers() {
    return [
      {
        source: "/:path*",
        headers: [
          { key: "Content-Security-Policy", value: csp },
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
          { key: "X-Frame-Options", value: "DENY" },
          {
            key: "Permissions-Policy",
            value: "camera=(), microphone=(), geolocation=(), payment=(), usb=()",
          },
          { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains" },
        ],
      },
    ];
  },
};

export default nextConfig;
