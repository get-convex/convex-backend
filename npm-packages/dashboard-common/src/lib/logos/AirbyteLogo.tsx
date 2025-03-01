import Image from "next/image";

export function AirbyteLogo({
  className,
  size,
}: {
  className?: string;
  size: number;
}) {
  return (
    <Image
      className={className}
      src="/airbyte.svg"
      height={size}
      width={size}
      alt="Airbyte logo"
    />
  );
}
