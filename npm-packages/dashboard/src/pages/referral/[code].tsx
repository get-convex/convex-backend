import { GetServerSideProps } from "next";
import { auth0 } from "server/auth0";
import { Flourish } from "layouts/LoginLayout";
import Head from "next/head";
import { useParams } from "next/navigation";
import Background from "components/login/images/background.svg";
import { ConvexLogo } from "dashboard-common/elements/ConvexLogo";
import { RedeemReferralLanding } from "components/referral/RedeemReferralLanding";

export const getServerSideProps: GetServerSideProps = async ({
  req,
  res,
  query,
}) => {
  try {
    // Check if user is authenticated without forcing login
    const session = await auth0().getSession(req, res);

    // If user is authenticated, redirect to the apply page
    if (session?.user) {
      return {
        redirect: {
          destination: `/referral/${query.code}/apply`,
          permanent: false,
        },
      };
    }

    // If not authenticated, render the page normally
    return { props: {} };
  } catch (error) {
    // Something went wrong with Auth0, so we’ll just render the logged out page
    console.error("Auth error:", error);
    return { props: {} };
  }
};

const title = "Someone thinks you’d like Convex!";
const description = "Get Convex resources for free with this referral code.";
const ogImage = `https://www.convex.dev/api/og?title=${encodeURIComponent(title)}`;

export default function ReferralLandingPage() {
  const { code } = useParams<{ code: string }>();

  return (
    <div className="flex h-screen w-full flex-col items-center bg-background-brand">
      <Head>
        <title>{title}</title>

        <meta name="description" content={description} />

        <meta property="og:title" content={title} />
        <meta property="og:description" content={description} />

        <meta property="og:type" content="website" />
        <meta property="og:site_name" content="Convex" />
        <meta
          property="og:url"
          content={`https://dashboard.convex.dev/referral/${code}`}
        />
        <meta property="og:image" content={ogImage} />

        <meta name="twitter:card" content="summary_large_image" />
        <meta name="twitter:title" content={title} />
        <meta name="twitter:description" content={description} />
        <meta name="twitter:image" content={ogImage} />
      </Head>

      <Flourish />

      <div className="mt-20">
        <ConvexLogo />
      </div>

      <div className="absolute left-1/2 top-36 hidden -translate-x-1/2 lg:block">
        <Background className="stroke-[#D7D7D7] dark:hidden" />
      </div>

      <RedeemReferralLanding title={title} code={code} />
    </div>
  );
}
