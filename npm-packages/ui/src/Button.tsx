import React, { forwardRef, useContext } from "react";
import classNames from "classnames";
import { tv } from "tailwind-variants";
import { Tooltip, TooltipSide } from "@ui/Tooltip";
import { UrlObject } from "url";
import { UIContext } from "@ui/UIContext";

export type ButtonProps = {
  children?: React.ReactNode;
  variant?: "primary" | "danger" | "neutral" | "unstyled";
  inline?: boolean;
  size?: "xs" | "sm" | "md" | "lg";
  focused?: boolean;
  icon?: React.ReactNode;
  className?: string;
  disabled?: boolean;
  tip?: React.ReactNode;
  tipSide?: TooltipSide;
} & (
  | React.ButtonHTMLAttributes<HTMLButtonElement>
  | {
      href: React.AnchorHTMLAttributes<HTMLAnchorElement>["href"] | UrlObject;
      onClick?: React.AnchorHTMLAttributes<HTMLAnchorElement>["onClick"];
      target?: React.AnchorHTMLAttributes<HTMLAnchorElement>["target"];
    }
);

export const Button = forwardRef<HTMLElement, ButtonProps>(function Button(
  {
    children,
    inline = false,
    variant = "primary",
    className = "",
    size = "sm",
    disabled = false,
    focused = false,
    icon,
    tip,
    tipSide,
    ...props
  },
  ref,
) {
  const Link = useContext(UIContext);
  const { href, onClick, target, type } =
    "href" in props
      ? { ...props, type: undefined }
      : { ...props, href: undefined, target: undefined };
  const buttonClassName =
    variant === "unstyled"
      ? className
      : buttonClasses({
          inline,
          icon,
          variant,
          disabled,
          focused,
          className,
          size,
        });
  if (href !== undefined && !disabled) {
    return (
      <Tooltip tip={tip} side={tipSide} wrapsButton>
        <Link
          passHref
          href={href}
          ref={ref as any}
          role="link"
          className={buttonClassName}
          tabIndex={0}
          target={target}
          onClick={onClick}
          {...(props as any)}
        >
          {icon && <div>{icon}</div>}
          {children}
        </Link>
      </Tooltip>
    );
  }
  return (
    <Tooltip tip={tip} side={tipSide} wrapsButton>
      {/* we're allowed to use button here. It's the Button component */}
      {/* eslint-disable-next-line react/forbid-elements */}
      <button
        // eslint-disable-next-line react/button-has-type
        type={type ?? "button"}
        tabIndex={0}
        onClick={onClick as any}
        className={buttonClassName}
        disabled={disabled}
        ref={ref as any}
        {...(props as any)}
      >
        {/* This needs to be wrapped in a dom element to 
          fix an issue with the google translate extension
          throwing errors when the icon switches between different icons.
          The negative margin is added when the icon doesn't exist 
          to not render the flex gap.
          https://github.com/facebook/react/issues/11538#issuecomment-390386520
       */}
        {icon && <div>{icon}</div>}
        {children}
      </button>
    </Tooltip>
  );
});

const button = tv({
  base: "inline-flex animate-fadeInFromLoading select-none items-center whitespace-nowrap rounded text-sm font-medium transition-colors focus-visible:border focus-visible:border-border-selected focus-visible:outline-none",
  variants: {
    variant: {
      primary:
        "border-util-accent bg-util-accent text-white hover:bg-util-accent/80",
      neutral: "text-content-primary hover:bg-background-primary",
      danger: "text-content-error hover:bg-background-errorSecondary",
    },
    icon: {
      true: "gap-1.5",
      false: "gap-2.5",
    },
    accent: {
      none: "border",
      inline: "",
    },
    disabled: {
      true: classNames(
        "cursor-not-allowed",
        "bg-neutral-1 border-neutral-1 text-neutral-4 hover:bg-neutral-1",
        "dark:bg-neutral-11 dark:border-neutral-11 dark:text-neutral-6 dark:hover:bg-neutral-11",
      ),
      false: "cursor-pointer",
    },
    focused: {
      true: "",
      false: "",
    },
    size: {
      xs: "p-1",
      sm: "px-2.5 py-2",
      md: "px-4 py-3",
      lg: "rounded-lg px-6 py-[1.125rem]",
    },
  },
  compoundVariants: [
    {
      variant: "primary",
      accent: "inline",
      class: "bg-transparent text-content-accent hover:bg-background-tertiary",
      disabled: false,
    },
    {
      variant: "primary",
      focused: true,
      accent: "none",
      class: "bg-util-accent/80 text-white",
      disabled: false,
    },
    {
      variant: "primary",
      focused: true,
      accent: "inline",
      class: "bg-background-tertiary",
      disabled: false,
    },
    {
      variant: "neutral",
      accent: "none",
      class: "bg-background-tertiary",
      disabled: false,
    },
    {
      variant: "neutral",
      accent: "none",
      class: "border bg-background-secondary hover:bg-background-tertiary",
      disabled: false,
    },
    {
      variant: "neutral",
      focused: true,
      accent: ["none", "inline"],
      class: "bg-background-primary",
      disabled: false,
    },
    {
      variant: "neutral",
      focused: true,
      accent: "none",
      class: "border border-border-selected bg-background-secondary",
      disabled: false,
    },
    {
      variant: "danger",
      accent: "none",
      class: "border-background-error bg-background-error",
      disabled: false,
    },
    {
      variant: "danger",
      focused: true,
      class: "bg-background-errorSecondary text-content-error",
      disabled: false,
    },
    {
      disabled: true,
      accent: "inline",
      class:
        "bg-transparent text-neutral-4 hover:bg-transparent dark:bg-transparent dark:text-neutral-6",
    },
  ],
  defaultVariants: {
    size: "sm",
    variant: "primary",
    focused: false,
    disabled: false,
    accent: "none",
    icon: false,
  },
});

export function buttonClasses({
  variant,
  disabled,
  inline,
  icon,
  focused,
  className,
  size,
}: Pick<
  ButtonProps,
  "variant" | "disabled" | "focused" | "className" | "size" | "inline" | "icon"
>) {
  return variant === "unstyled"
    ? className
    : button({
        accent: inline ? "inline" : "none",
        icon: !!icon,
        variant,
        disabled,
        focused,
        className,
        size,
      });
}
