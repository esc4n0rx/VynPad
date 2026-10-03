import { motion } from "motion/react";
import { NavLink } from "react-router";
import { NAV_ITEMS, SETTINGS_PATH, type NavItem } from "@/app/navigation";
import { ServerStatus, SlotDots } from "@/components/shell/ServerStatus";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

function SidebarLink({ item, collapsed }: { item: NavItem; collapsed: boolean }) {
  const Icon = item.icon;

  const link = (
    <NavLink
      to={item.path}
      end={item.path === "/"}
      aria-label={collapsed ? item.label : undefined}
      className={({ isActive }) =>
        cn(
          "relative flex h-8 items-center gap-2.5 rounded-md text-body font-medium focus-ring transition-colors",
          collapsed ? "w-8 justify-center" : "px-2.5",
          isActive ? "text-fg" : "text-fg-2 hover:text-fg",
        )
      }
    >
      {({ isActive }) => (
        <>
          {isActive && (
            <motion.span
              layoutId="sidebar-selection"
              transition={{ type: "spring", stiffness: 380, damping: 32 }}
              className="absolute inset-0 rounded-md bg-accent-soft"
            />
          )}
          <Icon className="relative size-4 shrink-0" strokeWidth={1.75} />
          {!collapsed && <span className="relative truncate">{item.label}</span>}
        </>
      )}
    </NavLink>
  );

  return collapsed ? (
    <Tooltip content={item.label} side="right">
      {link}
    </Tooltip>
  ) : (
    link
  );
}

/** Sidebar translúcida: 220 px, ou só ícones (56 px) em janelas estreitas. */
export function Sidebar({ collapsed }: { collapsed: boolean }) {
  const sections = NAV_ITEMS.filter((item) => item.path !== SETTINGS_PATH);
  const settings = NAV_ITEMS.filter((item) => item.path === SETTINGS_PATH);

  return (
    <nav
      aria-label="Seções"
      className={cn(
        "flex shrink-0 flex-col bg-sidebar backdrop-blur-(--blur)",
        collapsed ? "w-14 items-center" : "w-[220px]",
      )}
    >
      <ul className={cn("flex flex-col gap-0.5 pt-1", collapsed ? "items-center" : "px-3")}>
        {sections.map((item) => (
          <li key={item.path} className={cn(!collapsed && "w-full")}>
            <SidebarLink item={item} collapsed={collapsed} />
          </li>
        ))}
        <li aria-hidden className={cn("my-2 h-px bg-separator", collapsed ? "w-6" : "w-full")} />
        {settings.map((item) => (
          <li key={item.path} className={cn(!collapsed && "w-full")}>
            <SidebarLink item={item} collapsed={collapsed} />
          </li>
        ))}
      </ul>
      <div className="mt-auto">
        {collapsed ? (
          <div className="flex flex-col items-center gap-2 pb-4">
            <SlotDots className="flex-col" />
          </div>
        ) : (
          <ServerStatus />
        )}
      </div>
    </nav>
  );
}
