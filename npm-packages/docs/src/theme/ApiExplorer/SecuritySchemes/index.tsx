/*
 * Ejected to apply Markdown to authorization descriptions and link to a custom location.
 */

import React from "react";
import Markdown from "react-markdown";

import Link from "@docusaurus/Link";
import { useTypedSelector } from "@theme/ApiItem/hooks";

function Passthrough({ children }: any) {
  return children;
}

const AuthTypeLinks: Record<string, string> = {
  "OAuth Team Token": "/platform-apis/oauth-applications",
  "OAuth Project Token": "//platform-apis/oauth-applications",
  "Team Token": "/platform-apis#managing-your-own-projects",
  PAT: "/platform-apis#managing-your-own-projects",
};

function SecuritySchemes(props: any) {
  const options = useTypedSelector((state: any) => state.auth.options);
  const selected = useTypedSelector((state: any) => state.auth.selected);

  if (selected === undefined) return null;

  if (options[selected]?.[0]?.type === undefined) {
    return null;
  }

  const selectedAuth = options[selected];
  return (
    <details className="openapi-security__details" open={false}>
      <summary className="openapi-security__summary-container">
        <h4 className="openapi-security__summary-header">
          Authorization: {selectedAuth[0].name ?? selectedAuth[0].type}
        </h4>
      </summary>
      {selectedAuth.map((auth: any) => {
        const isHttp = auth.type === "http";
        const isApiKey = auth.type === "apiKey";
        const isOauth2 = auth.type === "oauth2";
        const isOpenId = auth.type === "openIdConnect";

        const infoAuthPath = `/${props.infoPath}#authentication`;
        if (isHttp) {
          if (auth.scheme === "bearer") {
            const {
              name,
              key,
              type: _type,
              scopes,
              scheme: _scheme,
              ...rest
            } = auth;
            return (
              <React.Fragment key={auth.key}>
                <pre
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    background: "var(--openapi-card-background-color)",
                    whiteSpace: "pre-wrap",
                    wordWrap: "break-word",
                    overflowWrap: "break-word",
                  }}
                >
                  <span>
                    <strong>
                      <Link to={AuthTypeLinks[key] || infoAuthPath}>
                        {name ?? key}
                      </Link>
                    </strong>
                  </span>
                  <span>HTTP Bearer token</span>
                  {scopes && scopes.length > 0 && (
                    <span>
                      <strong>scopes: </strong>
                      <code>
                        {auth.scopes.length > 0 ? auth.scopes.toString() : "[]"}
                      </code>
                    </span>
                  )}
                  {Object.keys(rest).map((k, _i) => {
                    return (
                      <span key={k}>
                        {typeof rest[k] === "object" ? (
                          JSON.stringify(rest[k], null, 2)
                        ) : typeof rest[k] === "string" &&
                          rest[k].includes("](") ? (
                          <Markdown components={{ a: Link, p: Passthrough }}>
                            {rest[k]}
                          </Markdown>
                        ) : (
                          String(rest[k])
                        )}
                      </span>
                    );
                  })}
                </pre>
              </React.Fragment>
            );
          }
          if (auth.scheme === "basic") {
            const { name, key, type, scopes, ...rest } = auth;
            return (
              <React.Fragment key={auth.key}>
                <pre
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    background: "var(--openapi-card-background-color)",
                  }}
                >
                  <span>
                    <strong>name:</strong>{" "}
                    <Link to={infoAuthPath}>{name ?? key}</Link>
                  </span>
                  <span>
                    <strong>type: </strong>
                    {type}
                  </span>
                  {scopes && scopes.length > 0 && (
                    <span>
                      <strong>scopes: </strong>
                      <code>
                        {auth.scopes.length > 0 ? auth.scopes.toString() : "[]"}
                      </code>
                    </span>
                  )}
                  {Object.keys(rest).map((k, _i) => {
                    return (
                      <span key={k}>
                        <strong>{k}: </strong>
                        {typeof rest[k] === "object"
                          ? JSON.stringify(rest[k], null, 2)
                          : String(rest[k])}
                      </span>
                    );
                  })}
                </pre>
              </React.Fragment>
            );
          }
          return (
            <React.Fragment key={auth.key}>
              <pre
                style={{
                  display: "flex",
                  flexDirection: "column",
                  background: "var(--openapi-card-background-color)",
                }}
              >
                <span>
                  <strong>name:</strong>{" "}
                  <Link to={infoAuthPath}>{auth.name ?? auth.key}</Link>
                </span>
                <span>
                  <strong>type: </strong>
                  {auth.type}
                </span>
                <span>
                  <strong>in: </strong>
                  {auth.in}
                </span>
              </pre>
            </React.Fragment>
          );
        }

        if (isApiKey) {
          const { name, key, type: _, scopes, ...rest } = auth;
          return (
            <React.Fragment key={auth.key}>
              <pre
                style={{
                  display: "flex",
                  flexDirection: "column",
                  background: "var(--openapi-card-background-color)",
                  whiteSpace: "pre-wrap",
                  wordWrap: "break-word",
                  overflowWrap: "break-word",
                }}
              >
                <span>
                  <strong>
                    <Link to={AuthTypeLinks[key] || infoAuthPath}>
                      {name ?? key}
                    </Link>
                  </strong>
                </span>
                <span>API Key in {auth.in}</span>
                {scopes && scopes.length > 0 && (
                  <span>
                    <strong>scopes: </strong>
                    <code>
                      {auth.scopes.length > 0 ? auth.scopes.toString() : "[]"}
                    </code>
                  </span>
                )}
                {Object.keys(rest).map((k, _i) => {
                  return (
                    <span key={k}>
                      {typeof rest[k] === "object" ? (
                        JSON.stringify(rest[k], null, 2)
                      ) : typeof rest[k] === "string" &&
                        rest[k].includes("](") ? (
                        <Markdown components={{ a: Link, p: Passthrough }}>
                          {rest[k]}
                        </Markdown>
                      ) : (
                        String(rest[k])
                      )}
                    </span>
                  );
                })}
              </pre>
            </React.Fragment>
          );
        }

        if (isOauth2) {
          const { name, key, type, scopes, flows, ...rest } = auth;
          return (
            <React.Fragment key={selected}>
              <pre
                style={{
                  display: "flex",
                  flexDirection: "column",
                  background: "var(--openapi-card-background-color)",
                }}
              >
                <span>
                  <strong>name:</strong>{" "}
                  <Link to={infoAuthPath}>{name ?? key}</Link>
                </span>
                <span>
                  <strong>type: </strong>
                  {type}
                </span>
                {scopes && scopes.length > 0 && (
                  <span>
                    <strong>scopes: </strong>
                    <code>
                      {auth.scopes.length > 0 ? auth.scopes.toString() : "[]"}
                    </code>
                  </span>
                )}
                {Object.keys(rest).map((k, _i) => {
                  return (
                    <span key={k}>
                      <strong>{k}: </strong>
                      {typeof rest[k] === "object"
                        ? JSON.stringify(rest[k], null, 2)
                        : String(rest[k])}
                    </span>
                  );
                })}
                {flows && (
                  <span>
                    <code>
                      <strong>flows: </strong>
                      {JSON.stringify(flows, null, 2)}
                    </code>
                  </span>
                )}
              </pre>
            </React.Fragment>
          );
        }

        if (isOpenId) {
          const { name, key, scopes, type, ...rest } = auth;
          return (
            <React.Fragment key={auth.key}>
              <pre
                style={{
                  display: "flex",
                  flexDirection: "column",
                  background: "var(--openapi-card-background-color)",
                }}
              >
                <span>
                  <strong>name:</strong>{" "}
                  <Link to={infoAuthPath}>{name ?? key}</Link>
                </span>
                <span>
                  <strong>type: </strong>
                  {type}
                </span>
                {scopes && scopes.length > 0 && (
                  <span>
                    <strong>scopes: </strong>
                    <code>
                      {auth.scopes.length > 0 ? auth.scopes.toString() : "[]"}
                    </code>
                  </span>
                )}
                {Object.keys(rest).map((k, _i) => {
                  return (
                    <span key={k}>
                      <strong>{k}: </strong>
                      {typeof rest[k] === "object"
                        ? JSON.stringify(rest[k], null, 2)
                        : String(rest[k])}
                    </span>
                  );
                })}
              </pre>
            </React.Fragment>
          );
        }

        return undefined;
      })}
    </details>
  );
}

export default SecuritySchemes;
