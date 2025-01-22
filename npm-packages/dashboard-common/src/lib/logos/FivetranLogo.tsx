import Image from "next/image";
import { useTheme } from "next-themes";

export function FivetranLogo({ className }: { className?: string }) {
  const { resolvedTheme: currentTheme } = useTheme();
  const image =
    currentTheme === "dark" ? "/fivetran-white.svg" : "/fivetran-blue.svg";
  return (
    <Image
      className={className}
      src={image}
      height="40"
      width="40"
      alt="Fivetran logo"
    />
  );
}
