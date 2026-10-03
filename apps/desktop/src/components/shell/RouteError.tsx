import { useState } from "react";
import { TriangleAlert } from "lucide-react";
import { useRouteError } from "react-router";
import { describeRouteError } from "@/components/shell/describeRouteError";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { log } from "@/lib/log";

/** Error boundary de rota: tela amigável no lugar da que quebrou, sem derrubar a sidebar. */
export function RouteError() {
  const error = useRouteError();
  const [copied, setCopied] = useState(false);
  const details = describeRouteError(error);

  const copy = () => {
    navigator.clipboard
      .writeText(details)
      .then(() => {
        setCopied(true);
      })
      .catch((e: unknown) => {
        log.warn(`não consegui copiar os detalhes do erro: ${String(e)}`);
      });
  };

  return (
    <div role="alert" className="flex h-full items-center justify-center">
      <EmptyState
        icon={<TriangleAlert strokeWidth={1.75} />}
        title="Algo deu errado nesta tela"
        description="O restante do app continua funcionando. Você pode voltar ao início ou copiar os detalhes para o suporte."
        action={
          <div className="flex gap-2">
            <Button variant="primary" onClick={copy}>
              {copied ? "Copiado" : "Copiar detalhes"}
            </Button>
            <Button
              onClick={() => {
                window.location.hash = "#/";
                window.location.reload();
              }}
            >
              Voltar ao início
            </Button>
          </div>
        }
      />
    </div>
  );
}
