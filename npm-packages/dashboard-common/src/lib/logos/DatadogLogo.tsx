import Image from "next/image";
import { useTheme } from "next-themes";
import DatadogColoredImage from "../../../public/dd_icon_rgb.png";
import DatadogWhiteImage from "../../../public/dd_icon_white.png";

type DatadogLogoProps = {
  className?: string;
};

export function DatadogLogo({ className }: DatadogLogoProps) {
  const { resolvedTheme: currentTheme } = useTheme();
  const image =
    currentTheme === "dark" ? DatadogWhiteImage : DatadogColoredImage;
  return <Image className={className} src={image} alt="Datadog logo" />;
}
