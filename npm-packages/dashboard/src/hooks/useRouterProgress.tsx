import Router from "next/router";
import NProgress from "nprogress";
import "nprogress/nprogress.css";
import { useEffect } from "react";

NProgress.configure({ showSpinner: false });

export const useRouterProgress = () => {
  useEffect(() => {
    const startProgress = (event: any, { shallow }: { shallow: boolean }) =>
      !shallow && NProgress.start();
    const endProgress = (event: any, { shallow }: { shallow: boolean }) =>
      !shallow && NProgress.done();
    Router.events.on("routeChangeStart", startProgress);
    Router.events.on("routeChangeComplete", endProgress);
    Router.events.on("routeChangeError", endProgress);
    return () => {
      Router.events.off("routeChangeStart", startProgress);
      Router.events.off("routeChangeComplete", endProgress);
      Router.events.off("routeChangeError", endProgress);
    };
  }, []);
};
