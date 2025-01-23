import Image from "next/image";
import AxiomPng from "../../../public/axiom.png";

export function AxiomLogo({ className }: { className?: string }) {
  return <Image className={className} src={AxiomPng} alt="Axiom logo" />;
}
