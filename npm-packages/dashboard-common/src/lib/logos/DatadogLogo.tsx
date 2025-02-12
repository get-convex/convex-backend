import Image from "next/image";
import { useTheme } from "next-themes";

type DatadogLogoProps = {
  className?: string;
};

export function DatadogLogo({ className }: DatadogLogoProps) {
  const { resolvedTheme: currentTheme } = useTheme();
  const image =
    currentTheme === "dark" ? "/dd_icon_white.png" : "/dd_icon_rgb.png";
  return (
    <Image
      className={className}
      src={image}
      alt="Datadog logo"
      width={16}
      height={16}
    />
  );
}
