import Image from "next/image";
import { useTheme } from "next-themes";

export function SentryLogo({ className }: { className?: string }) {
  const { resolvedTheme: currentTheme } = useTheme();
  const image =
    currentTheme === "dark" ? "/sentry-dark.png" : "/sentry-light.png";
  return <Image className={className} src={image} alt="Sentry logo" />;
}
