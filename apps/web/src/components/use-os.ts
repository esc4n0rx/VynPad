"use client";

import { useSyncExternalStore } from "react";
import { detectOs, type Os } from "@/lib/os";

type UaData = { platform?: string };

let cached: Os | undefined;

function read(): Os {
  cached ??= detectOs(
    navigator.userAgent,
    (navigator as Navigator & { userAgentData?: UaData }).userAgentData?.platform,
  );
  return cached;
}

const subscribe = () => () => undefined;

/** O sistema de quem visita. No servidor (e na hidratação) é `other`; depois passa ao real. */
export function useOs(): Os {
  return useSyncExternalStore(subscribe, read, () => "other");
}
