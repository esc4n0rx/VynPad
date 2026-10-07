import { resolveDownload } from "@/lib/download";

// O resultado muda com o release e, no repositório privado, o endereço assinado expira em
// segundos: nada aqui pode ser guardado em cache.
export const dynamic = "force-dynamic";

const NO_STORE = { "Cache-Control": "no-store", "X-Robots-Tag": "noindex" } as const;

export async function GET(
  request: Request,
  context: { params: Promise<{ platform: string }> },
): Promise<Response> {
  const { platform } = await context.params;
  const outcome = await resolveDownload(platform);

  if (outcome.ok) {
    return new Response(null, { status: 302, headers: { ...NO_STORE, Location: outcome.url } });
  }

  // Quem clicou no botão é uma pessoa: leva de volta à seção de download com o aviso. Quem pede
  // JSON (script, monitoramento) recebe o código do erro.
  if (request.headers.get("accept")?.includes("text/html")) {
    const back = new URL("/", request.url);
    back.hash = "download";
    back.searchParams.set("download", outcome.code === "upstream" ? "error" : "unavailable");
    return new Response(null, { status: 303, headers: { ...NO_STORE, Location: back.toString() } });
  }
  return Response.json({ error: outcome.code }, { status: outcome.status, headers: NO_STORE });
}
