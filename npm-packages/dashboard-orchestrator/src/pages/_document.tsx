import Document, {
  DocumentContext,
  DocumentInitialProps,
  Head,
  Html,
  Main,
  NextScript,
} from "next/document";
import { getServerRuntimeConfig, RuntimeConfig } from "../lib/config";

type Props = DocumentInitialProps & {
  config: RuntimeConfig;
};

export default class MyDocument extends Document<Props> {
  static async getInitialProps(ctx: DocumentContext): Promise<Props> {
    const initial = await Document.getInitialProps(ctx);
    return { ...initial, config: getServerRuntimeConfig() };
  }

  render() {
    const { config } = this.props;
    return (
      <Html>
        <Head>
          {/*
           * Inject runtime config before any script that might read it.
           * Read by `orchestratorUrl()` in src/lib/config.ts on the
           * client. The publish-time image bakes nothing; the script's
           * contents come from server-side env at request time.
           */}
          <script
            id="__convex_runtime_config__"
            dangerouslySetInnerHTML={{
              __html: `window.__CONVEX_CONFIG__=${JSON.stringify(config)};`,
            }}
          />
        </Head>
        <body>
          <Main />
          <NextScript />
        </body>
      </Html>
    );
  }
}
