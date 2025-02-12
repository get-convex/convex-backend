import Image from "next/image";

export function AxiomLogo({ className }: { className?: string }) {
  return (
    <Image
      className={className}
      src="/axiom.png"
      alt="Axiom logo"
      width={16}
      height={16}
    />
  );
}
