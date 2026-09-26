import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";

// Tipos escritos à mão por enquanto; a T-506 passa a gerá-los com tauri-specta.
export interface AppInfo {
  app_version: string;
  core_version: string;
}

export function useAppInfo() {
  return useQuery({
    queryKey: ["app_info"],
    queryFn: () => invoke<AppInfo>("app_info"),
    staleTime: Infinity,
  });
}
