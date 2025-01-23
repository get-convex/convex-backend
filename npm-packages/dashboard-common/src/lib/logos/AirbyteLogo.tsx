import Image from "next/image";

export function AirbyteLogo({ className }: { className?: string }) {
  return (
    <Image
      className={className}
      src="/airbyte.svg"
      height="40"
      width="40"
      alt="Airbyte logo"
    />
  );
}
