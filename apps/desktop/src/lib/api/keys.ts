/** Chaves do React Query, num só lugar: os eventos do backend invalidam por elas. */
export const keys = {
  appInfo: ["app_info"] as const,
  settings: ["settings"] as const,
  profiles: ["profiles"] as const,
  profile: (id: string) => ["profiles", id] as const,
  devices: ["devices"] as const,
  connectionLog: ["connection_log"] as const,
  nearby: ["nearby"] as const,
  serverInfo: ["server_info"] as const,
  driver: ["driver"] as const,
  xinput: ["xinput"] as const,
  networkInterfaces: ["network_interfaces"] as const,
};
