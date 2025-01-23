import { ImageProps } from "next/legacy/image";
import Logo from "images/convex-light.svg";
import { cn } from "lib/cn";

type LogoProps = Omit<ImageProps, "src">;

export function ConvexLogo({ width, className = "", ...props }: LogoProps) {
  return (
    <Logo
      width={width ?? 228}
      className={cn("fill-black dark:fill-white", className)}
      alt="Convex Logo"
      {...props}
    />
  );
}
