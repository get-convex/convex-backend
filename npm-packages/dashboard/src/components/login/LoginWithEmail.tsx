import { ArrowRightIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import { Button } from "@ui/Button";

import { useRouter } from "next/router";
import { UIProvider } from "@ui/UIContext";

export function LoginWithEmail({ returnTo }: { returnTo?: string }) {
  const { query } = useRouter();
  return query.allowUsernameAuth || process.env.NODE_ENV !== "production" ? (
    <UIProvider>
      <Button
        href={`/api/auth/login?useEmail=true${
          returnTo ? `&returnTo=${returnTo}` : ""
        }`}
        variant="neutral"
        className={classNames("mx-auto mt-8 mb-2 z-20")}
      >
        <span className="mr-2">Log in with Email</span>
        <ArrowRightIcon />
      </Button>
    </UIProvider>
  ) : null;
}
