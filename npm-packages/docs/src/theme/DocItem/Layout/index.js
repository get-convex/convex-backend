import React from "react";
import Layout from "@theme-original/DocItem/Layout";
import Footer from "@theme-original/Footer";

export default function LayoutWrapper(props) {
  return (
    <>
      <Layout {...props} />
      <Footer />
    </>
  );
}
