import Image from "next/image";
import { useTheme } from "next-themes";

export function PostHogLogo({
  className,
  size,
}: {
  className?: string;
  size: number;
}) {
  const { resolvedTheme: currentTheme } = useTheme();
  const image =
    currentTheme === "dark"
      ? "/posthog-icon-light.svg"
      : "/posthog-icon.svg";
  return (
    <Image
      className={className}
      src={image}
      alt="PostHog logo"
      width={size}
      height={size}
    />
  );
}
