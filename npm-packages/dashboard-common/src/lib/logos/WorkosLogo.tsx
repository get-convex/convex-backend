import Image from "next/image";
import { useTheme } from "next-themes";

export function WorkosLogo({
  className,
  size,
}: {
  className?: string;
  size: number;
}) {
  const { resolvedTheme: currentTheme } = useTheme();
  const image =
    currentTheme === "dark" ? "/workos-blue.svg" : "/workos-blue.svg";
  return (
    <Image
      className={className}
      src={image}
      height={size}
      width={size}
      alt="Fivetran logo"
    />
  );
}
