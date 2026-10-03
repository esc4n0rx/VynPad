import type { DriverStatusDto } from "@/lib/bindings";

/** Texto curto do driver de controles virtuais. */
export function driverLabel(status: DriverStatusDto | undefined): string {
  switch (status?.state) {
    case undefined:
      return "Driver…";
    case "ready":
      return "Driver OK";
    case "not_installed":
      return "Driver ausente";
    case "outdated":
      return "Driver desatualizado";
    case "service_stopped":
      return "Driver parado";
    case "error":
      return "Driver com erro";
  }
}
