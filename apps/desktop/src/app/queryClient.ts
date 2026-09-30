import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";
import { notifyError } from "@/lib/toast";

declare module "@tanstack/react-query" {
  interface Register {
    mutationMeta: {
      /** O chamador trata o erro (ex.: pede confirmação); não mostrar o aviso automático. */
      handlesErrors?: boolean;
    };
  }
}

/** O `QueryClient` do app: todo erro de command vira um aviso com a mensagem traduzida. */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    queryCache: new QueryCache({ onError: notifyError }),
    mutationCache: new MutationCache({
      onError: (error, _variables, _context, mutation) => {
        if (mutation.meta?.handlesErrors) return;
        notifyError(error);
      },
    }),
    defaultOptions: {
      // Dados vêm do backend local: sem refetch agressivo por foco de janela.
      queries: { refetchOnWindowFocus: false, retry: 1 },
    },
  });
}
