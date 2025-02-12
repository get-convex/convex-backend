import Image from "next/image";

export function AxiomLogo({ className }: { className?: string }) {
  return <Image className={className} src="/axiom.svg" alt="Axiom logo" />;
}
