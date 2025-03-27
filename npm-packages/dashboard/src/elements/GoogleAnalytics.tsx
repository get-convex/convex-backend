import Script from "next/script";

export function GoogleAnalytics() {
  return (
    <>
      <Script src="https://www.googletagmanager.com/gtag/js?id=G-BE1B7P7T72" />
      <Script id="google-analytics">{`
        window.dataLayer = window.dataLayer || [];
        function gtag(){
          dataLayer.push(arguments);
        }
        gtag('js', new Date());
      
        gtag('config', 'G-BE1B7P7T72');
      `}</Script>
    </>
  );
}

export function fireGoogleAnalyticsEvent(eventName: string) {
  window.gtag("event", eventName, {
    transaction_id: "convex-account",
  });
}
