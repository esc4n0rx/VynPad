import type { ParseKeys } from "i18next";
import type { DriverStatusDto } from "@/lib/bindings";

/** Chave (namespace `common`) do texto curto do driver de controles virtuais. */
export function driverLabelKey(status: DriverStatusDto | undefined): ParseKeys<"common"> {
  switch (status?.state) {
    case undefined:
      return "driver.loading";
    case "ready":
      return "driver.ready";
    case "not_installed":
      return "driver.notInstalled";
    case "outdated":
      return "driver.outdated";
    case "service_stopped":
      return "driver.stopped";
    case "error":
      return "driver.error";
  }
}
