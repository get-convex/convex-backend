import { LoginLayout } from "layouts/LoginLayout";
import { useAuth0 } from "hooks/useAuth0";
import classNames from "classnames";
import { Snippet } from "@common/elements/Snippet";
import { Loading } from "@ui/Loading";
import { Button } from "@ui/Button";
import { GoogleAnalytics } from "elements/GoogleAnalytics";
import { useRouter } from "next/router";
import { useAccessToken } from "hooks/useServerSideData";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

// TODO -- gtag & etc on this page.
function Auth() {
  const { isAuthenticated } = useAuth0();
  const router = useRouter();

  const [accessToken] = useAccessToken();

  if (isAuthenticated) {
    return (
      <LoginLayout>
        {/* emit the account_created event */}
        <GoogleAnalytics />
        <DisplayAccessToken accessToken={accessToken} />
        <Button
          variant="neutral"
          onClick={() => {
            void router.push("/api/auth/logout");
          }}
          className={classNames("mt-4 ml-auto")}
        >
          Log Out
        </Button>
      </LoginLayout>
    );
  }
  return <Loading />;
}

function DisplayAccessToken({ accessToken }: { accessToken: string }) {
  return (
    <div className="max-w-prose text-sm text-content-primary">
      Paste the token below to <code>convex dev</code> that you ran yourself or
      in an embedded editor on a <code>convex.dev</code> site. Never paste it
      anywhere you don't trust!
      <Snippet className="my-6" value={accessToken} copying="Access token" />
    </div>
  );
}

export default withAuthenticatedPage(Auth);
