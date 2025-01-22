import Image from "next/image";
import { useTheme } from "next-themes";
import SentryDark from "../../../public/sentry-dark.png";
import SentryLight from "../../../public/sentry-light.png";

export function SentryLogo({ className }: { className?: string }) {
  const { resolvedTheme: currentTheme } = useTheme();
  const image = currentTheme === "dark" ? SentryLight : SentryDark;
  return <Image className={className} src={image} alt="Sentry logo" />;
}
