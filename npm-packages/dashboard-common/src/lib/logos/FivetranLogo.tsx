import Image from "next/image";
import { useTheme } from "next-themes";

export function FivetranLogo({
  className,
  size,
}: {
  className?: string;
  size: number;
}) {
  const { resolvedTheme: currentTheme } = useTheme();
  const image =
    currentTheme === "dark" ? "/fivetran-white.svg" : "/fivetran-blue.svg";
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
