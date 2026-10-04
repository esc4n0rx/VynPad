import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type AppSettings } from "@/lib/bindings";
import { keys } from "./keys";

/** As configurações completas (o backend sempre devolve todos os campos). */
export type Settings = Required<AppSettings>;

/** Configurações; o evento `settingsChanged` mantém o cache em dia. */
export function useSettings() {
  return useQuery({
    queryKey: keys.settings,
    queryFn: async () => (await commands.getSettings()) as Settings,
    staleTime: Infinity,
  });
}

/** Grava as configurações (validadas no backend). */
export function useUpdateSettings() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (settings: Settings) => (await commands.updateSettings(settings)) as Settings,
    onSuccess: (saved) => {
      queryClient.setQueryData(keys.settings, saved);
    },
  });
}

/**
 * Grava uma alteração parcial sobre as configurações atuais (as demais ficam como estão).
 * Devolve a função de gravar e se há uma gravação em andamento.
 */
export function useSettingsPatch() {
  const queryClient = useQueryClient();
  const update = useUpdateSettings();
  const patch = (change: Partial<Settings>) => {
    const current = queryClient.getQueryData<Settings>(keys.settings);
    if (current) update.mutate({ ...current, ...change });
  };
  return { patch, isPending: update.isPending };
}
