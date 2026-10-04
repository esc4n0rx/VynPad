import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { create } from "zustand";
import { commands, type SessionHealthDto, type SlotsDto } from "@/lib/bindings";
import { keys } from "./keys";

interface SlotsState {
  /** Os 4 slots como o backend os enviou por último (`null` até a primeira leitura). */
  slots: SlotsDto | null;
  /** Estado do fluxo de input de cada slot ("Reconectando…" em `stalled`). */
  health: Partial<Record<number, SessionHealthDto>>;
  setSlots: (slots: SlotsDto) => void;
  setHealth: (slot: number, health: SessionHealthDto) => void;
  clearHealth: (slot: number) => void;
}

/** Dados de alta frequência dos slots, alimentados pelos eventos (registrados uma vez no boot). */
export const useSlots = create<SlotsState>((set) => ({
  slots: null,
  health: {},
  setSlots: (slots) => {
    set((s) => {
      // Slot que ficou livre perde o estado de saúde antigo.
      const health = { ...s.health };
      for (const slot of slots.slots) {
        if (slot.state === "free") delete health[slot.slot];
      }
      return { slots, health };
    });
  },
  setHealth: (slot, health) => {
    set((s) => ({ health: { ...s.health, [slot]: health } }));
  },
  clearHealth: (slot) => {
    set((s) => {
      const health = { ...s.health };
      delete health[slot];
      return { health };
    });
  },
}));

/** Desconecta quem está no slot (`bye{kicked}`) e o libera. */
export function useDisconnectSlot() {
  return useMutation({ mutationFn: (slot: number) => commands.disconnectSlot(slot) });
}

/** Pausa ou retoma o slot. */
export function useSetSlotPaused() {
  return useMutation({
    mutationFn: (args: { slot: number; paused: boolean }) =>
      commands.setSlotPaused(args.slot, args.paused),
  });
}

/** Troca dois slots de lugar (o controle, a sessão e o perfil vão juntos). */
export function useSwapSlots() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: { a: number; b: number }) => commands.swapSlots(args.a, args.b),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.slotProfiles });
    },
  });
}

/** O `id` do perfil de cada slot, na ordem P1–P4 (`null` onde nunca houve dispositivo). */
export function useSlotProfiles() {
  return useQuery({ queryKey: keys.slotProfiles, queryFn: () => commands.getSlotProfiles() });
}

/** Troca o perfil do slot na hora; vale até o dispositivo desconectar. */
export function useSetSlotProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: { slot: number; profileId: string }) =>
      commands.setSlotProfile(args.slot, args.profileId),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.slotProfiles });
    },
  });
}
