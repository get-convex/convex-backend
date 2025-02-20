import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import Head from "next/head";
import { LoginLayout } from "layouts/LoginLayout";
import { RegisterApplication } from "components/RegisterApplication";

export { getServerSideProps } from "lib/ssr";

function OAuthProviderRegistration() {
  return (
    <div className="h-screen">
      <Head>
        <title>Register Convex OAuth Application</title>
      </Head>
      <LoginLayout>
        <RegisterApplication />
      </LoginLayout>
    </div>
  );
}

export default withAuthenticatedPage(OAuthProviderRegistration);
