import Link from "next/link";
import { ExternalLinkIcon } from "@radix-ui/react-icons";

export function LearnMoreLink({ name, link }: { name: string; link: string }) {
  return (
    <div className="mb-2 px-4 text-xs text-content-primary sm:px-6">
      Learn more about{" "}
      <Link
        passHref
        href={link}
        className="inline-flex items-center text-content-link"
        target="_blank"
      >
        {name}
        <ExternalLinkIcon className="ml-0.5 h-3 w-3" />
      </Link>
    </div>
  );
}
