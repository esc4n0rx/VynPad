import { useState } from "react";
import { TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useRouteError } from "react-router";
import { describeRouteError } from "@/components/shell/describeRouteError";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { log } from "@/lib/log";

const HOME_HASH = "#/";

/** Error boundary de rota: tela amigável no lugar da que quebrou, sem derrubar a sidebar. */
export function RouteError() {
  const { t } = useTranslation();
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
        title={t("routeError.title")}
        description={t("routeError.description")}
        action={
          <div className="flex gap-2">
            <Button variant="primary" onClick={copy}>
              {copied ? t("routeError.copied") : t("routeError.copy")}
            </Button>
            <Button
              onClick={() => {
                window.location.hash = HOME_HASH;
                window.location.reload();
              }}
            >
              {t("routeError.home")}
            </Button>
          </div>
        }
      />
    </div>
  );
}
