import Image from "next/image";
import { useTheme } from "next-themes";

export function SentryLogo({
  className,
  size,
}: {
  className?: string;
  size: number;
}) {
  const { resolvedTheme: currentTheme } = useTheme();
  const image =
    currentTheme === "dark" ? "/sentry-light.png" : "/sentry-dark.png";
  return (
    <Image
      className={className}
      src={image}
      alt="Sentry logo"
      width={size}
      height={size}
    />
  );
}
