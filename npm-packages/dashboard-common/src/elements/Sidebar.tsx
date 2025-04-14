import classNames from "classnames";
import { useRouter } from "next/router";
import { ReactNode } from "react";
import { useWindowSize } from "react-use";
import omit from "lodash/omit";
import {
  DoubleArrowLeftIcon,
  DoubleArrowRightIcon,
} from "@radix-ui/react-icons";
import { Tooltip } from "@ui/Tooltip";
import { Button } from "@ui/Button";

export type SidebarItem = {
  key: string | null;
  label: string;
  Icon: React.FC<{ className?: string }>;
  href: string;
  isActive?: (currentPage: string) => boolean;
  disabled?: boolean;
  tooltip?: string;
};

export type SidebarGroup = {
  key: string;
  items: SidebarItem[];
};

export function Sidebar({
  items,
  collapsed,
  setCollapsed,
}: {
  collapsed: boolean;
  setCollapsed: (collapsed: boolean) => void;
  items: SidebarGroup[];
}) {
  const router = useRouter();

  const path = router.pathname
    .replace("/t/[team]/[project]/[deploymentName]", "")
    .split("/")
    .filter((i) => !!i);
  const currentPage = path[0] ?? null;

  const { width } = useWindowSize();

  return (
    <aside
      className={classNames(
        "bg-background-secondary animate-fadeInFromLoading",
        "shadow border-b sm:border-b-0 sm:border-r transition-[min-width]",
        "px-3 py-2 w-0 overflow-auto scrollbar-none",
        "z-40 w-full min-h-fit sm:w-fit sm:h-full",
        "flex flex-row sm:flex-col justify-between",
        "shrink-0",
        { [`min-w-[20px]`]: collapsed },
        { [`min-w-[130px]`]: !collapsed },
      )}
    >
      <div className="flex gap-1 sm:flex-col sm:divide-x-0 sm:divide-y">
        {items.map((group) => (
          <div key={group.key} className="flex gap-1 sm:flex-col sm:py-2">
            {group.items.map((item) => (
              <div className="relative h-[1.875rem]" key={item.key}>
                <Tooltip
                  tip={item.tooltip ? item.tooltip : collapsed && item.label}
                  side={width > 640 ? "right" : "bottom"}
                  className="text-left"
                >
                  <SidebarLink
                    {...omit(item, "key")}
                    collapsed={collapsed}
                    isActive={currentPage === item.key}
                    disabled={item.disabled}
                    small
                  >
                    {item.label}
                  </SidebarLink>
                </Tooltip>
              </div>
            ))}
          </div>
        ))}
      </div>
      <Button
        variant="unstyled"
        onClick={() => setCollapsed(!collapsed)}
        aria-label={collapsed ? "Expand" : "Collapse"}
        tip={collapsed ? "Expand" : undefined}
        tipSide="right"
        className={classNames(
          sidebarLinkClassNames({
            isHoverable: true,
            small: true,
          }),
          "sm:flex hidden",
        )}
        icon={collapsed ? <DoubleArrowRightIcon /> : <DoubleArrowLeftIcon />}
      >
        {!collapsed && "Collapse"}
      </Button>
    </aside>
  );
}

export function SidebarLink({
  collapsed,
  href,
  query,
  children,
  Icon,
  isActive,
  disabled,
  proBadge,
  small,
  tip,
  target,
}: {
  collapsed?: boolean;
  href: string;
  query?: Record<string, string>;
  children: ReactNode;
  isActive: boolean;
  Icon?: React.FC<{ className?: string }>;
  disabled?: boolean;
  proBadge?: boolean;
  small?: boolean;
  tip?: string;
  target?: "_blank";
}) {
  const { query: currentQuery } = useRouter();
  return (
    <Button
      tip={tip}
      variant="unstyled"
      href={
        disabled
          ? undefined
          : {
              pathname: href,
              query: currentQuery.component
                ? { ...query, component: currentQuery.component }
                : query,
            }
      }
      aria-disabled={disabled}
      className={sidebarLinkClassNames({
        isActive,
        isDisabled: disabled,
        small,
      })}
      target={target}
    >
      {Icon && (
        <Icon
          className={classNames(
            "size-[1.125rem] shrink-0",
            !collapsed && "text-content-secondary",
          )}
          aria-hidden
        />
      )}
      <span className={classNames("select-none flex-1", collapsed && "hidden")}>
        {children}
      </span>
      {proBadge && (
        <span
          className="rounded bg-util-accent px-1.5 py-0.5 text-xs font-semibold uppercase tracking-wider text-white"
          title="Only available in paid plans"
        >
          Pro
        </span>
      )}
    </Button>
  );
}

export function sidebarLinkClassNames(props: {
  // defaults to false
  isActive?: boolean;
  // defaults to true
  isHoverable?: boolean;
  // default to false
  isDisabled?: boolean;
  // defaults to normal sans font
  font?: "mono";
  small?: boolean;
  // defaults to true
  fitWidth?: boolean;
}) {
  let fontSize = props.small ? "text-[13px]" : "text-sm";
  if (props.font === "mono") {
    fontSize = props.small ? "text-xs" : "text-[0.8125rem]";
  }
  return classNames(
    "w-full",
    "rounded flex items-center gap-2 whitespace-nowrap",
    "text-content-primary",
    fontSize,
    (props.fitWidth ?? true) ? "min-w-fit" : null,
    props.font === "mono" && "font-mono px-1 py-1",
    props.small ? "p-1.5" : "px-3 py-2",
    !props.isDisabled && (props.isHoverable ?? true)
      ? "focus-visible:ring cursor-pointer hover:bg-background-primary"
      : null,
    "focus-visible:outline-0 focus-visible:ring-1 focus-visible:ring-util-accent/40 rounded focus-visible:ring-offset-2",
    (props.isActive ?? false) ? "font-semibold bg-background-tertiary" : null,
    props.isDisabled
      ? "text-content-tertiary cursor-not-allowed w-fit text-left"
      : null,
  );
}
