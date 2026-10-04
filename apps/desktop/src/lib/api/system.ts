import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "@/lib/bindings";
import { isAppError } from "@/lib/errors";
import { keys } from "./keys";

/** Estado do firewall do Windows para o app (a leitura usa COM e leva um instante). */
export function useFirewallStatus() {
  return useQuery({
    queryKey: keys.firewall,
    queryFn: () => commands.firewallStatus(),
    staleTime: 30_000,
  });
}

/**
 * Cria as regras do app no firewall (pede o UAC). Recusar o UAC não é falha do app: a tela
 * trata `cancelled` e este erro não vira aviso automático.
 */
export function useFixFirewall() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => commands.fixFirewall(),
    meta: { handlesErrors: true },
    onSuccess: (status) => {
      queryClient.setQueryData(keys.firewall, status);
    },
  });
}

/** `true` quando o erro é o usuário recusando o pedido de administrador. */
export function isCancelled(error: unknown): boolean {
  return isAppError(error) && error.code === "cancelled";
}

/** "Testar conexão": sondas no canal de controle e na descoberta, pela interface de LAN. */
export function useTestConnection() {
  return useMutation({ mutationFn: () => commands.testConnection() });
}

export function useOpenNetworkSettings() {
  return useMutation({ mutationFn: () => commands.openNetworkSettings() });
}

export function useOpenLogsDir() {
  return useMutation({ mutationFn: () => commands.openLogsDir() });
}

/** Onde ficam os logs (a tela Sobre mostra o caminho). */
export function useLogsDir() {
  return useQuery({
    queryKey: keys.logsDir,
    queryFn: () => commands.logsDir(),
    staleTime: Infinity,
  });
}

export function useOpenVigemDownload() {
  return useMutation({ mutationFn: () => commands.openVigemDownload() });
}

/** Apaga o histórico de conexões; devolve quantos registros saíram. */
export function useClearHistory() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => commands.clearHistory(),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.connectionLog });
    },
  });
}

/** Gera uma identidade nova e esquece os dispositivos pareados. */
export function useRegenerateIdentity() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => commands.regenerateIdentity(),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: keys.devices }),
        queryClient.invalidateQueries({ queryKey: keys.serverInfo }),
      ]);
    },
  });
}

export function useRestartApp() {
  return useMutation({ mutationFn: () => commands.restartApp() });
}

/** Exporta o pacote de diagnóstico (sem segredos); devolve o caminho do `.zip`. */
export function useExportDiagnostics() {
  return useMutation({ mutationFn: (maskIps: boolean) => commands.exportDiagnostics(maskIps) });
}
