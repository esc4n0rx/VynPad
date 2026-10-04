import { useMutation, useQuery } from "@tanstack/react-query";
import { create } from "zustand";
import { commands, type ApprovalPromptDto } from "@/lib/bindings";
import { keys } from "./keys";

/** Portas, endereços de LAN e slots livres do server. */
export function useServerInfo() {
  return useQuery({ queryKey: keys.serverInfo, queryFn: () => commands.getServerInfo() });
}

/** Aparelhos anunciando na rede; o evento `devicesDiscovered` atualiza o cache. */
export function useNearbyDevices() {
  return useQuery({
    queryKey: keys.nearby,
    queryFn: () => commands.getNearbyDevices(),
    staleTime: Infinity,
  });
}

/** Status do driver de controles virtuais; o evento `driverChanged` mantém em dia. */
export function useDriverStatus() {
  return useQuery({
    queryKey: keys.driver,
    queryFn: () => commands.driverStatus(),
    staleTime: Infinity,
  });
}

/** Quem ocupa os 4 índices XInput (a tela pergunta a 1 Hz enquanto está aberta). */
export function useXinputSlots(enabled = true) {
  return useQuery({
    queryKey: keys.xinput,
    queryFn: () => commands.xinputSlots(),
    refetchInterval: enabled ? 1000 : false,
    enabled,
  });
}

/** Interfaces de rede ativas (Configurações). */
export function useNetworkInterfaces() {
  return useQuery({
    queryKey: keys.networkInterfaces,
    queryFn: () => commands.listNetworkInterfaces(),
  });
}

/** Gera o QR code e o PIN. Cada chamada invalida o PIN anterior. */
export function useStartPairing() {
  return useMutation({ mutationFn: () => commands.startPairing() });
}

export function useCancelPairing() {
  return useMutation({ mutationFn: () => commands.cancelPairing() });
}

/** O último dispositivo que terminou o primeiro pareamento (`seq` sobe a cada um). */
interface PairedState {
  seq: number;
  last: { device_id: string; name: string; model: string } | null;
  record: (device: { device_id: string; name: string; model: string }) => void;
}

/** A tela de parear fecha com a animação de sucesso quando `seq` passa do que ela viu ao abrir. */
export const useLastPaired = create<PairedState>((set) => ({
  seq: 0,
  last: null,
  record: (device) => {
    set((s) => ({ seq: s.seq + 1, last: device }));
  },
}));

/** Pedidos de confirmação de novos dispositivos esperando resposta. */
interface ApprovalsState {
  pending: ApprovalPromptDto[];
  add: (prompt: ApprovalPromptDto) => void;
  remove: (requestId: number) => void;
}

export const useApprovals = create<ApprovalsState>((set) => ({
  pending: [],
  add: (prompt) => {
    set((s) => ({
      pending: [...s.pending.filter((p) => p.request_id !== prompt.request_id), prompt],
    }));
  },
  remove: (requestId) => {
    set((s) => ({ pending: s.pending.filter((p) => p.request_id !== requestId) }));
  },
}));

/** Responde ao popup. O pedido some da lista quando o backend confirma (`approvalResolved`). */
export function useAnswerApproval() {
  return useMutation({
    mutationFn: (args: { requestId: number; accept: boolean }) =>
      commands.answerApproval(args.requestId, args.accept),
  });
}
