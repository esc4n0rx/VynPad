import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type ProfileInputDto } from "@/lib/bindings";
import { keys } from "./keys";

export function useProfiles() {
  return useQuery({ queryKey: keys.profiles, queryFn: () => commands.listProfiles() });
}

export function useProfile(id: string | null) {
  return useQuery({
    queryKey: keys.profile(id ?? ""),
    queryFn: () => commands.getProfile(id ?? ""),
    enabled: id !== null,
  });
}

/** Cria (`id` nulo) ou salva um perfil. Salvar aplica na hora nos controles que o usam. */
export function useSaveProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: { id: string | null; input: ProfileInputDto }) =>
      commands.saveProfile(args.id, args.input),
    onSuccess: async (saved) => {
      // O editor lê o perfil do cache ao reabrir: mantém o que acabou de ser gravado.
      queryClient.setQueryData(keys.profile(saved.summary.id), saved);
      await queryClient.invalidateQueries({ queryKey: keys.profiles });
    },
  });
}

/**
 * Apaga um perfil. Em uso, o backend responde `profile_in_use` (com `in_use`): a tela pede
 * confirmação e chama de novo com `confirm: true`. Por isso este erro não vira aviso automático.
 */
export function useDeleteProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: { id: string; confirm: boolean }) =>
      commands.deleteProfile(args.id, args.confirm),
    meta: { handlesErrors: true },
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: keys.profiles }),
        queryClient.invalidateQueries({ queryKey: keys.devices }),
      ]);
    },
  });
}

export function useDuplicateProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: { id: string; name: string | null }) =>
      commands.duplicateProfile(args.id, args.name),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.profiles });
    },
  });
}

/** Importa o conteúdo de um `.padprofile`. */
export function useImportProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (text: string) => commands.importProfile(text),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.profiles });
    },
  });
}

/** Devolve o texto do `.padprofile` para a tela salvar em arquivo. */
export function useExportProfile() {
  return useMutation({ mutationFn: (id: string) => commands.exportProfile(id) });
}

export function useRestoreDefaultProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => commands.restoreDefaultProfile(),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.profiles });
    },
  });
}
