import classNames from "classnames";
import Image from "next/image";

type AvatarSize = "small" | "medium" | "large";

export type AvatarProps = {
  name?: string;
  size?: AvatarSize;
  isSystem?: boolean;
};

const classesForSize: Record<AvatarSize, string> = {
  small: "w-4 h-4",
  medium: "w-9 h-9",
  large: "w-12 h-12",
};

export function Avatar({
  name = "",
  size = "medium",
  isSystem = false,
}: AvatarProps) {
  const initial = name.charAt(0);

  return (
    <span
      className={classNames(
        "inline-flex items-center select-none justify-center rounded-lg bg-background-tertiary",
        classesForSize[size],
      )}
    >
      {isSystem ? (
        <Image src="/convex-logo-only.svg" width="14" height="14" alt="" />
      ) : (
        <span
          className={`${size === "small" ? "text-xs" : "text-sm"} font-medium uppercase leading-none`}
        >
          {initial}
        </span>
      )}
    </span>
  );
}
