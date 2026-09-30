import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type DeviceDefaultsDto, type LogQueryDto } from "@/lib/bindings";
import { keys } from "./keys";

/** Dispositivos pareados. Os eventos `devicesChanged`/`devicesPaired` invalidam esta consulta. */
export function useDevices() {
  return useQuery({ queryKey: keys.devices, queryFn: () => commands.listDevices() });
}

function useDeviceMutation<T>(run: (args: T) => Promise<unknown>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: run,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.devices });
    },
  });
}

export function useRenameDevice() {
  return useDeviceMutation((args: { id: string; name: string }) =>
    commands.renameDevice(args.id, args.name),
  );
}

/** Revoga: o token deixa de valer e a sessão em andamento é encerrada. */
export function useRevokeDevice() {
  return useDeviceMutation((id: string) => commands.revokeDevice(id));
}

/** Esquece: apaga o registro e o histórico; o aparelho precisa parear de novo. */
export function useForgetDevice() {
  return useDeviceMutation((id: string) => commands.forgetDevice(id));
}

export function useSetDeviceDefaults() {
  return useDeviceMutation((args: { id: string; defaults: DeviceDefaultsDto }) =>
    commands.setDeviceDefaults(args.id, args.defaults),
  );
}

/** Histórico de conexões (mais recentes primeiro). */
export function useConnectionLog(query: LogQueryDto) {
  return useQuery({
    queryKey: [...keys.connectionLog, query] as const,
    queryFn: () => commands.getConnectionLog(query),
  });
}
