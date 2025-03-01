import Image from "next/image";

export function AxiomLogo({
  className,
  size,
}: {
  className?: string;
  size: number;
}) {
  return (
    <Image
      className={className}
      src="/axiom.png"
      alt="Axiom logo"
      width={size}
      height={size}
    />
  );
}
