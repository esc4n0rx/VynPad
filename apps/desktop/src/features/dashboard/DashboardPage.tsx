import { Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useDevices } from "@/lib/api/devices";
import { useProfiles } from "@/lib/api/profiles";
import { useServerInfo, useXinputSlots } from "@/lib/api/server";
import {
  useDisconnectSlot,
  useSetSlotPaused,
  useSetSlotProfile,
  useSlotProfiles,
  useSlots,
  useSwapSlots,
  useTestRumble,
} from "@/lib/api/slots";
import { useToasts } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { SlotCard } from "./SlotCard";
import { slotCardKind } from "./slotView";

/** Os 4 controles: quem está jogando em cada um, ao vivo. */
export function DashboardPage() {
  const { t } = useTranslation("dashboard");
  const navigate = useNavigate();
  const slots = useSlots((s) => s.slots);
  const health = useSlots((s) => s.health);
  const server = useServerInfo();
  const devices = useDevices();
  const profiles = useProfiles();
  const slotProfiles = useSlotProfiles();
  const xinput = useXinputSlots();
  const swap = useSwapSlots();
  const setProfile = useSetSlotProfile();
  const pause = useSetSlotPaused();
  const disconnect = useDisconnectSlot();
  const testRumble = useTestRumble();

  const address = server.data?.addresses[0];
  const goPair = () => {
    void navigate("/devices");
  };

  const swapSlots = (a: number, b: number) => {
    swap.mutate(
      { a, b },
      {
        onSuccess: () => {
          useToasts.getState().push("success", t("swapped", { from: a + 1, to: b + 1 }));
        },
      },
    );
  };

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <header className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <h1 className="text-title font-semibold tracking-tight">{t("title")}</h1>
        <span className="flex items-center gap-2 text-body text-fg-2">
          <span
            aria-hidden
            className={cn("size-2 rounded-full", server.data ? "bg-success" : "bg-fg-3")}
          />
          {server.data
            ? t("server.online")
            : server.isError
              ? t("server.offline")
              : t("server.starting")}
          {server.data && (
            <span className="text-fg-3 tabular-nums">
              {address
                ? t("server.address", { ip: address.ip, port: server.data.control_port })
                : t("server.noAddress")}
            </span>
          )}
        </span>
        <span className="flex-1" />
        <Button variant="primary" onClick={goPair}>
          <Plus />
          {t("pairNew")}
        </Button>
      </header>

      <section
        aria-label={t("grid")}
        className="grid min-h-0 flex-1 grid-cols-1 content-start gap-4 min-[860px]:grid-cols-2"
      >
        {slots === null
          ? [0, 1, 2, 3].map((index) => <Skeleton key={index} className="h-72 rounded-lg" />)
          : slots.slots.map((slot) => (
              <SlotCard
                key={slot.slot}
                slot={slot}
                kind={slotCardKind(slot, health[slot.slot], xinput.data)}
                health={health[slot.slot]}
                device={devices.data?.find((d) => d.id === slot.device_id)}
                profiles={profiles.data ?? []}
                profileId={slotProfiles.data?.[slot.slot] ?? null}
                onPair={goPair}
                onProfile={(profileId) => {
                  setProfile.mutate({ slot: slot.slot, profileId });
                }}
                onPause={(paused) => {
                  pause.mutate({ slot: slot.slot, paused });
                }}
                onDisconnect={() => {
                  disconnect.mutate(slot.slot);
                }}
                onTestRumble={() => {
                  testRumble.mutate(slot.slot);
                }}
                onLatency={() => {
                  void navigate("/latency");
                }}
                onSwap={(other) => {
                  swapSlots(slot.slot, other);
                }}
              />
            ))}
      </section>
    </div>
  );
}
