import { useQuery } from "@tanstack/react-query";
import { commands } from "@/lib/bindings";
import { keys } from "./keys";

export function useAppInfo() {
  return useQuery({
    queryKey: keys.appInfo,
    queryFn: () => commands.appInfo(),
    staleTime: Infinity,
  });
}
