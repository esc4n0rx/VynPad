import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "@/lib/bindings";
import { keys } from "./keys";

/**
 * A atualização que o backend já encontrou (ao abrir, a cada 6 h ou pelo botão). O evento
 * `update:available` põe o resultado aqui; ler o command cobre a janela que abriu depois.
 */
export function usePendingUpdate() {
  return useQuery({
    queryKey: keys.pendingUpdate,
    queryFn: () => commands.pendingUpdate(),
    staleTime: Infinity,
  });
}

/** "Procurar atualizações": consulta o servidor agora, com o automático ligado ou não. */
export function useCheckForUpdate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => commands.checkForUpdate(),
    onSuccess: (update) => {
      queryClient.setQueryData(keys.pendingUpdate, update);
    },
  });
}

/**
 * Baixa e instala. Com controles conectados e `confirmed = false` o backend não faz nada e
 * devolve quantos são (`started = false`), para a tela pedir a confirmação.
 */
export function useInstallUpdate() {
  return useMutation({
    mutationFn: (confirmed: boolean) => commands.installUpdate(confirmed),
  });
}
