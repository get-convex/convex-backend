/* eslint-disable  no-self-assign */
import { query } from "../_generated/server";
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";

/**
 * The goal of this test is to run our V8 implementation of URL and URLSearchParams
 * and assert that it behaves as expected.
 *
 * The test cases were pulled from Deno and the goal is to either pass the test
 * case or throw an error with an appropriate error message for the things
 * we're intentionally skipping implementing.
 * https://github.com/denoland/deno/blob/10e4b2e14046b74469f7310c599579a6611513fe/cli/tests/unit/url_test.ts
 *
 * Known limitations:
 *  - URLs with scheme other than `http` and `https` are unsupported (CX-3087)
 *  - setting the host is unsupported (CX-3090)
 */

function urlParsing() {
  const url = new URL("https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat");
  assert.strictEqual(url.hash, "#qat");
  assert.strictEqual(url.host, "baz.qat:8000");
  assert.strictEqual(url.hostname, "baz.qat");
  assert.strictEqual(
    url.href,
    "https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat",
  );
  assert.strictEqual(url.origin, "https://baz.qat:8000");
  assert.strictEqual(url.pathname, "/qux/quux");
  assert.strictEqual(url.port, "8000");
  assert.strictEqual(url.protocol, "https:");
  assert.strictEqual(url.search, "?foo=bar&baz=12");
  assert.deepEqual(url.searchParams.getAll("foo"), ["bar"]);
  assert.deepEqual(url.searchParams.getAll("baz"), ["12"]);
  assert.strictEqual(
    String(url),
    "https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat",
  );
}

function urlProtocolParsing() {
  assert.strictEqual(new URL("http://foo").protocol, "http:");
  assert.strictEqual(new URL("https://foo").protocol, "https:");

  assert.throws(() => new URL("1://foo"), TypeError, "Invalid URL: '1://foo'");
  assert.throws(() => new URL("+://foo"), TypeError, "Invalid URL: '+://foo'");
  assert.throws(() => new URL("-://foo"), TypeError, "Invalid URL: '-://foo'");
  assert.throws(() => new URL(".://foo"), TypeError, "Invalid URL: '.://foo'");
  assert.throws(() => new URL("_://foo"), TypeError, "Invalid URL: '_://foo'");
  assert.throws(() => new URL("=://foo"), TypeError, "Invalid URL: '=://foo'");
  assert.throws(() => new URL("!://foo"), TypeError, "Invalid URL: '!://foo'");
  assert.throws(() => new URL(`"://foo`), TypeError, `Invalid URL: '"://foo'`);
  assert.throws(() => new URL("$://foo"), TypeError, "Invalid URL: '$://foo'");
  assert.throws(() => new URL("%://foo"), TypeError, "Invalid URL: '%://foo'");
  assert.throws(() => new URL("^://foo"), TypeError, "Invalid URL: '^://foo'");
  assert.throws(() => new URL("*://foo"), TypeError, "Invalid URL: '*://foo'");
  assert.throws(() => new URL("*://foo"), TypeError, "Invalid URL: '*://foo'");
}

function urlAuthenticationParsing() {
  const specialUrl = new URL("http://foo:bar@baz");
  assert.strictEqual(specialUrl.username, "foo");
  assert.strictEqual(specialUrl.password, "bar");
  assert.strictEqual(specialUrl.hostname, "baz");
  assert.throws(() => new URL("file://foo:bar@baz"), TypeError, "Invalid URL");
  // non http/https protocols not supported yet
  // const nonSpecialUrl = new URL("abcd://foo:bar@baz");
  // assert.strictEqual(nonSpecialUrl.username, "foo");
  // assert.strictEqual(nonSpecialUrl.password, "bar");
  // assert.strictEqual(nonSpecialUrl.hostname, "baz");
}

function urlHostnameParsing() {
  // IPv6.
  assert.strictEqual(new URL("http://[::1]").hostname, "[::1]");
  // unsupported scheme
  //   assert.strictEqual(new URL("file://[::1]").hostname, "[::1]");
  //   assert.strictEqual(new URL("abcd://[::1]").hostname, "[::1]");
  assert.strictEqual(
    new URL("http://[0:f:0:0:f:f:0:0]").hostname,
    "[0:f::f:f:0:0]",
  );

  // Forbidden host code point.
  assert.throws(() => new URL("http:// a"), TypeError, "Invalid URL");
  assert.throws(() => new URL("file:// a"), TypeError, "Invalid URL");
  assert.throws(() => new URL("abcd:// a"), TypeError, "Invalid URL");
  assert.throws(() => new URL("http://%"), TypeError, "Invalid URL");
  assert.throws(() => new URL("file://%"), TypeError, "Invalid URL");
  // unsupported scheme
  //   assert.strictEqual(new URL("abcd://%").hostname, "%");

  // Percent-decode.
  assert.strictEqual(new URL("http://%21").hostname, "!");
  // unsupported scheme
  //   assert.strictEqual(new URL("file://%21").hostname, "!");
  //   assert.strictEqual(new URL("abcd://%21").hostname, "%21");

  // IPv4 parsing.
  assert.strictEqual(new URL("http://260").hostname, "0.0.1.4");
  // unsupported scheme
  //   assert.strictEqual(new URL("file://260").hostname, "0.0.1.4");
  //   assert.strictEqual(new URL("abcd://260").hostname, "260");
  assert.strictEqual(new URL("http://255.0.0.0").hostname, "255.0.0.0");
  assert.throws(() => new URL("http://256.0.0.0"), TypeError, "Invalid URL");
  assert.strictEqual(new URL("http://0.255.0.0").hostname, "0.255.0.0");
  assert.throws(() => new URL("http://0.256.0.0"), TypeError, "Invalid URL");
  assert.strictEqual(new URL("http://0.0.255.0").hostname, "0.0.255.0");
  assert.throws(() => new URL("http://0.0.256.0"), TypeError, "Invalid URL");
  assert.strictEqual(new URL("http://0.0.0.255").hostname, "0.0.0.255");
  assert.throws(() => new URL("http://0.0.0.256"), TypeError, "Invalid URL");
  assert.strictEqual(new URL("http://0.0.65535").hostname, "0.0.255.255");
  assert.throws(() => new URL("http://0.0.65536"), TypeError, "Invalid URL");
  assert.strictEqual(new URL("http://0.16777215").hostname, "0.255.255.255");
  assert.throws(() => new URL("http://0.16777216"), TypeError, "Invalid URL");
  assert.strictEqual(new URL("http://4294967295").hostname, "255.255.255.255");
  assert.throws(() => new URL("http://4294967296"), TypeError, "Invalid URL");
}

function urlPortParsing() {
  const specialUrl = new URL("http://foo:8000");
  assert.strictEqual(specialUrl.hostname, "foo");
  assert.strictEqual(specialUrl.port, "8000");
  assert.throws(() => new URL("file://foo:8000"), TypeError, "Invalid URL");
  // unsupported scheme
  //   const nonSpecialUrl = new URL("abcd://foo:8000");
  //   assert.strictEqual(nonSpecialUrl.hostname, "foo");
  //   assert.strictEqual(nonSpecialUrl.port, "8000");
}

function urlModifications() {
  const url = new URL("https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat");
  url.hash = "";
  assert.strictEqual(url.href, "https://baz.qat:8000/qux/quux?foo=bar&baz=12");

  // Set host not implemented
  //   url.host = "qat.baz:8080";
  //   assert.strictEqual(url.href, "https://qat.baz:8080/qux/quux?foo=bar&baz=12");

  url.hostname = "foo.bar";
  assert.strictEqual(url.href, "https://foo.bar:8000/qux/quux?foo=bar&baz=12");

  // password / username unsupported
  //   url.password = "qux";
  //   assert.strictEqual(
  //     url.href,
  //     "https://foo:qux@foo.bar:8080/qux/quux?foo=bar&baz=12"
  //   );
  url.pathname = "/foo/bar%qat";
  assert.strictEqual(
    url.href,
    "https://foo.bar:8000/foo/bar%qat?foo=bar&baz=12",
  );
  url.port = "";
  assert.strictEqual(url.href, "https://foo.bar/foo/bar%qat?foo=bar&baz=12");
  url.protocol = "http:";
  assert.strictEqual(url.href, "http://foo.bar/foo/bar%qat?foo=bar&baz=12");
  url.search = "?foo=bar&foo=baz";
  assert.strictEqual(url.href, "http://foo.bar/foo/bar%qat?foo=bar&foo=baz");
  assert.deepEqual(url.searchParams.getAll("foo"), ["bar", "baz"]);
  // password / username unsupported
  //   url.username = "foo@bar";
  //   assert.strictEqual(
  //     url.href,
  //     "http://foo%40bar:qux@foo.bar/foo/bar%qat?foo=bar&foo=baz"
  //   );
  url.searchParams.set("bar", "qat");
  assert.strictEqual(
    url.href,
    "http://foo.bar/foo/bar%qat?foo=bar&foo=baz&bar=qat",
  );
  url.searchParams.delete("foo");
  assert.strictEqual(url.href, "http://foo.bar/foo/bar%qat?bar=qat");
  url.searchParams.append("foo", "bar");
  assert.strictEqual(url.href, "http://foo.bar/foo/bar%qat?bar=qat&foo=bar");
}

function urlModifyHref() {
  const url = new URL("http://example.com/");
  url.href = "https://example.com:8080/baz/qat#qux";
  assert.strictEqual(url.protocol, "https:");
  // password / username unsupported
  //   assert.strictEqual(url.username, "foo");
  //   assert.strictEqual(url.password, "bar");
  assert.strictEqual(url.host, "example.com:8080");
  assert.strictEqual(url.hostname, "example.com");
  assert.strictEqual(url.pathname, "/baz/qat");
  assert.strictEqual(url.hash, "#qux");
}

function urlModifyHrefErroring() {
  const url = new URL("http://example.com/");
  assert.throws(
    () => (url.href = "foo"),
    /^Could not parse URL: http:\/\/example.com\/$/,
  );
}

function urlNormalize() {
  const url = new URL("http://example.com");
  assert.strictEqual(url.pathname, "/");
  assert.strictEqual(url.href, "http://example.com/");
}

function urlModifyPathname() {
  const url = new URL("http://foo.bar/baz%qat/qux%quux");
  assert.strictEqual(url.pathname, "/baz%qat/qux%quux");
  // Self-assignment is to invoke the setter.
  url.pathname = url.pathname;
  assert.strictEqual(url.pathname, "/baz%qat/qux%quux");
  url.pathname = "baz#qat qux";
  assert.strictEqual(url.pathname, "/baz%23qat%20qux");
  url.pathname = url.pathname;
  assert.strictEqual(url.pathname, "/baz%23qat%20qux");
  url.pathname = "\\a\\b\\c";
  assert.strictEqual(url.pathname, "/a/b/c");
}

function urlModifyHash() {
  const url = new URL("http://foo.bar");
  url.hash = "%foo bar/qat%qux#bar";
  assert.strictEqual(url.hash, "#%foo%20bar/qat%qux#bar");
  url.hash = url.hash;
  assert.strictEqual(url.hash, "#%foo%20bar/qat%qux#bar");
}

function urlSearchParamsReuse() {
  const url = new URL("https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat");
  const sp = url.searchParams;
  url.hostname = "baz.qat";
  if (sp !== url.searchParams) {
    throw new Error("Search params should be reused.");
  }
}

function urlBackSlashes() {
  const url = new URL("https:\\\\baz.qat:8000\\qux\\quux?foo=bar&baz=12#qat");
  assert.strictEqual(
    url.href,
    "https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat",
  );
}

function urlProtocolSlashes() {
  assert.strictEqual(new URL("http:foo").href, "http://foo/");
  assert.strictEqual(new URL("http://foo").href, "http://foo/");
  // unsupported scheme
  // assert.strictEqual(new URL("file:foo").href, "file:///foo");
  // assert.strictEqual(new URL("file://foo").href, "file://foo/");
  // assert.strictEqual(new URL("abcd:foo").href, "abcd:foo");
  // assert.strictEqual(new URL("abcd://foo").href, "abcd://foo");
}

function urlRequireHost() {
  // unsupported scheme
  // assert.strictEqual(new URL("file:///").href, "file:///");
  assert.throws(() => new URL("ftp:///"), TypeError, "Invalid URL");
  assert.throws(() => new URL("http:///"), TypeError, "Invalid URL");
  assert.throws(() => new URL("https:///"), TypeError, "Invalid URL");
  assert.throws(() => new URL("ws:///"), TypeError, "Invalid URL");
  assert.throws(() => new URL("wss:///"), TypeError, "Invalid URL");
}

// function urlDriveLetter() {
//   assert.strictEqual(new URL("file:///C:").href, "file:///C:");
//   assert.strictEqual(new URL("file:///C:/").href, "file:///C:/");
//   assert.strictEqual(new URL("file:///C:/..").href, "file:///C:/");

//   // Don't recognise drive letters with extra leading slashes.
//   // FIXME(nayeemrmn): This is true according to
//   // https://jsdom.github.io/whatwg-url/#url=ZmlsZTovLy8vQzovLi4=&base=ZmlsZTovLy8=
//   // but not the behavior of rust-url.
//   // assert.strictEqual(new URL("file:////C:/..").href, "file:///");

//   // Drop the hostname if a drive letter is parsed.
//   assert.strictEqual(new URL("file://foo/C:").href, "file:///C:");

//   // Don't recognise drive letters in non-file protocols.
//   // FIXME(nayeemrmn): This is true according to
//   // https://jsdom.github.io/whatwg-url/#url=YWJjZDovL2Zvby9DOi8uLg==&base=ZmlsZTovLy8=
//   // but not the behavior of rust-url.
//   // assert.strictEqual(new URL("http://foo/C:/..").href, "http://foo/");
//   // assert.strictEqual(new URL("abcd://foo/C:/..").href, "abcd://foo/");
// }

function urlHostnameUpperCase() {
  assert.strictEqual(new URL("http://EXAMPLE.COM").href, "http://example.com/");
  // assert.strictEqual(new URL("abcd://EXAMPLE.COM").href, "abcd://EXAMPLE.COM");
}

function urlEmptyPath() {
  assert.strictEqual(new URL("http://foo").pathname, "/");
  // assert.strictEqual(new URL("file://foo").pathname, "/");
  // assert.strictEqual(new URL("abcd://foo").pathname, "");
}

function urlPathRepeatedSlashes() {
  assert.strictEqual(new URL("http://foo//bar//").pathname, "//bar//");
  // unsupported scheme
  // assert.strictEqual(new URL("file://foo///bar//").pathname, "/bar//");
  // assert.strictEqual(new URL("abcd://foo//bar//").pathname, "//bar//");
}

function urlTrim() {
  assert.strictEqual(
    new URL(" http://example.com  ").href,
    "http://example.com/",
  );
}

function urlEncoding() {
  // password / username unsupported
  // assert.strictEqual(
  //   new URL("http://a !$&*()=,;+'\"@example.com").username,
  //   "a%20!$&*()%3D,%3B+'%22"
  // );
  // assert.strictEqual(
  //   new URL("http://:a !$&*()=,;+'\"@example.com").password,
  //   "a%20!$&*()%3D,%3B+'%22"
  // );

  // https://url.spec.whatwg.org/#idna
  assert.strictEqual(new URL("http://mañana/c?d#e").hostname, "xn--maana-pta");
  // unsupported scheme
  // assert.strictEqual(new URL("abcd://mañana/c?d#e").hostname, "ma%C3%B1ana");
  assert.strictEqual(
    new URL("http://example.com/a ~!@$&*()=:/,;+'\"\\").pathname,
    "/a%20~!@$&*()=:/,;+'%22/",
  );
  assert.strictEqual(
    new URL("http://example.com?a ~!@$&*()=:/,;?+'\"\\").search,
    "?a%20~!@$&*()=:/,;?+%27%22\\",
  );

  // unsupported scheme
  // assert.strictEqual(
  //   new URL("abcd://example.com?a ~!@$&*()=:/,;?+'\"\\").search,
  //   "?a%20~!@$&*()=:/,;?+'%22\\"
  // );
  assert.strictEqual(
    new URL("http://example.com#a ~!@#$&*()=:/,;?+'\"\\").hash,
    "#a%20~!@#$&*()=:/,;?+'%22\\",
  );
}

function urlBase() {
  assert.strictEqual(
    new URL("d", new URL("http://foo/a?b#c")).href,
    "http://foo/d",
  );

  assert.strictEqual(
    new URL("", "http://foo/a/b?c#d").href,
    "http://foo/a/b?c",
  );
  // assert.strictEqual(new URL("", "file://foo/a/b?c#d").href, "file://foo/a/b?c");
  // assert.strictEqual(new URL("", "abcd://foo/a/b?c#d").href, "abcd://foo/a/b?c");

  assert.strictEqual(
    new URL("#e", "http://foo/a/b?c#d").href,
    "http://foo/a/b?c#e",
  );
  // assert.strictEqual(new URL("#e", "file://foo/a/b?c#d").href, "file://foo/a/b?c#e");
  // assert.strictEqual(new URL("#e", "abcd://foo/a/b?c#d").href, "abcd://foo/a/b?c#e");

  assert.strictEqual(
    new URL("?e", "http://foo/a/b?c#d").href,
    "http://foo/a/b?e",
  );
  // assert.strictEqual(new URL("?e", "file://foo/a/b?c#d").href, "file://foo/a/b?e");
  // assert.strictEqual(new URL("?e", "abcd://foo/a/b?c#d").href, "abcd://foo/a/b?e");

  assert.strictEqual(new URL("e", "http://foo/a/b?c#d").href, "http://foo/a/e");
  // assert.strictEqual(new URL("e", "file://foo/a/b?c#d").href, "file://foo/a/e");
  // assert.strictEqual(new URL("e", "abcd://foo/a/b?c#d").href, "abcd://foo/a/e");

  assert.strictEqual(new URL(".", "http://foo/a/b?c#d").href, "http://foo/a/");
  // assert.strictEqual(new URL(".", "file://foo/a/b?c#d").href, "file://foo/a/");
  // assert.strictEqual(new URL(".", "abcd://foo/a/b?c#d").href, "abcd://foo/a/");

  assert.strictEqual(new URL("..", "http://foo/a/b?c#d").href, "http://foo/");
  // assert.strictEqual(new URL("..", "file://foo/a/b?c#d").href, "file://foo/");
  // assert.strictEqual(new URL("..", "abcd://foo/a/b?c#d").href, "abcd://foo/");

  assert.strictEqual(new URL("/e", "http://foo/a/b?c#d").href, "http://foo/e");
  // assert.strictEqual(new URL("/e", "file://foo/a/b?c#d").href, "file://foo/e");
  // assert.strictEqual(new URL("/e", "abcd://foo/a/b?c#d").href, "abcd://foo/e");

  assert.strictEqual(
    new URL("//bar", "http://foo/a/b?c#d").href,
    "http://bar/",
  );
  // assert.strictEqual(new URL("//bar", "file://foo/a/b?c#d").href, "file://bar/");
  // assert.strictEqual(new URL("//bar", "abcd://foo/a/b?c#d").href, "abcd://bar");

  // assert.strictEqual(new URL("efgh:", "http://foo/a/b?c#d").href, "efgh:");
  // assert.strictEqual(new URL("efgh:", "file://foo/a/b?c#d").href, "efgh:");
  // assert.strictEqual(new URL("efgh:", "abcd://foo/a/b?c#d").href, "efgh:");

  // assert.strictEqual(new URL("/foo", "abcd:/").href, "abcd:/foo");
}

// function urlDriveLetterBase() {
//   assert.strictEqual(new URL("/b", "file:///C:/a/b").href, "file:///C:/b");
//   assert.strictEqual(new URL("/D:", "file:///C:/a/b").href, "file:///D:");
// }

// function urlSameProtocolBase() {
//   assert.strictEqual(new URL("http:", "http://foo/a").href, "http://foo/a");
//   // assert.strictEqual(new URL("file:", "file://foo/a").href, "file://foo/a");
//   // assert.strictEqual(new URL("abcd:", "abcd://foo/a").href, "abcd:");

//   assert.strictEqual(new URL("http:b", "http://foo/a").href, "http://foo/b");
//   // assert.strictEqual(new URL("file:b", "file://foo/a").href, "file://foo/b");
//   // assert.strictEqual(new URL("abcd:b", "abcd://foo/a").href, "abcd:b");
// }

function deletingAllParamsRemovesQuestionMarkFromURL() {
  const url = new URL("http://example.com/?param1&param2");
  url.searchParams.delete("param1");
  url.searchParams.delete("param2");
  assert.strictEqual(url.href, "http://example.com/");
  assert.strictEqual(url.search, "");
}

function removingNonExistentParamRemovesQuestionMarkFromURL() {
  const url = new URL("http://example.com/?");
  assert.strictEqual(url.href, "http://example.com/?");
  url.searchParams.delete("param1");
  assert.strictEqual(url.href, "http://example.com/");
  assert.strictEqual(url.search, "");
}

function sortingNonExistentParamRemovesQuestionMarkFromURL() {
  const url = new URL("http://example.com/?");
  assert.strictEqual(url.href, "http://example.com/?");
  url.searchParams.sort();
  assert.strictEqual(url.href, "http://example.com/");
  assert.strictEqual(url.search, "");
}

// function protocolNotHttpOrFile() {
//   const url = new URL("about:blank");
//   assert.strictEqual(url.href, "about:blank");
//   assert.strictEqual(url.protocol, "about:");
//   assert.strictEqual(url.origin, "null");
// }

function throwForInvalidPortConstructor() {
  const urls = [
    // If port is greater than 2^16 − 1, validation error, return failure.
    `https://baz.qat:${2 ** 16}`,
    "https://baz.qat:-32",
    "https://baz.qat:deno",
    "https://baz.qat:9land",
    "https://baz.qat:10.5",
  ];

  for (const url of urls) {
    assert.throws(() => new URL(url), TypeError, "Invalid URL");
  }

  // Do not throw for 0 & 65535
  new URL("https://baz.qat:65535");
  new URL("https://baz.qat:0");
}

function doNotOverridePortIfInvalid() {
  const initialPort = "3000";
  const url = new URL(`https://deno.land:${initialPort}`);
  // If port is greater than 2^16 − 1, validation error, return failure.
  url.port = `${2 ** 16}`;
  assert.strictEqual(url.port, initialPort);
}

function emptyPortForSchemeDefaultPort() {
  const nonDefaultPort = "3500";

  // unsupported scheme
  // const url = new URL("ftp://baz.qat:21");
  // assert.strictEqual(url.port, "");
  // url.port = nonDefaultPort;
  // assert.strictEqual(url.port, nonDefaultPort);
  // url.port = "21";
  // assert.strictEqual(url.port, "");
  // url.protocol = "http";
  // assert.strictEqual(url.port, "");

  const url2 = new URL("https://baz.qat:443");
  assert.strictEqual(url2.port, "");
  url2.port = nonDefaultPort;
  assert.strictEqual(url2.port, nonDefaultPort);
  url2.port = "443";
  assert.strictEqual(url2.port, "");
  url2.protocol = "http";
  assert.strictEqual(url2.port, "");
}

function assigningPortPropertyAffectsReceiverOnly() {
  // Setting `.port` should update only the receiver.
  const u1 = new URL("http://google.com/");
  const u2 = new URL(u1 as any);
  u2.port = "123";
  assert.strictEqual(u1.port, "");
  assert.strictEqual(u2.port, "123");
}

function urlSearchParamsIdentityPreserved() {
  // URLSearchParams identity should not be lost when URL is updated.
  const u = new URL("http://foo.com/");
  const sp1 = u.searchParams;
  u.href = "http://bar.com/?baz=42";
  const sp2 = u.searchParams;
  if (sp1 !== sp2) {
    throw new Error("Search params should be reused.");
  }
}

function urlTakeURLObjectAsParameter() {
  const url = new URL(
    new URL("https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat"),
  );
  assert.strictEqual(
    url.href,
    "https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat",
  );
}

function urlConsoleLog() {
  console.log(new URL("https://baz.qat:8000/qux/quux?foo=bar&baz=123#qat"));
}

export default query(async () => {
  return await wrapInTests({
    urlParsing,
    urlProtocolParsing,
    // unsupported scheme
    // protocolNotHttpOrFile,

    urlAuthenticationParsing,
    urlHostnameParsing,
    urlPortParsing,
    urlModifications,
    urlModifyHref,
    urlModifyHrefErroring,
    urlNormalize,
    urlModifyPathname,
    urlModifyHash,
    urlSearchParamsReuse,
    urlBackSlashes,
    urlProtocolSlashes,
    urlRequireHost,

    // We don't support the file scheme
    // urlDriveLetter,
    urlHostnameUpperCase,
    urlEmptyPath,
    urlPathRepeatedSlashes,
    urlTrim,
    urlEncoding,

    urlBase,
    // urlDriveLetterBase,
    // urlSameProtocolBase,
    deletingAllParamsRemovesQuestionMarkFromURL,
    removingNonExistentParamRemovesQuestionMarkFromURL,
    sortingNonExistentParamRemovesQuestionMarkFromURL,
    throwForInvalidPortConstructor,
    doNotOverridePortIfInvalid,
    emptyPortForSchemeDefaultPort,
    assigningPortPropertyAffectsReceiverOnly,
    urlTakeURLObjectAsParameter,
    urlSearchParamsIdentityPreserved,
    urlConsoleLog,
  });
});

export const passwordNotImplemented = query(() => {
  const url = new URL("https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat");
  url.password;
});
export const usernameNotImplemented = query(() => {
  const url = new URL("https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat");
  url.username;
});

export const unsupportUrlUsernameAndPassword = query(() => {
  new URL("http://foo:bar@baz");
});

export const unsupportedUrlProtocol = query(() => {
  new URL("file:///foo");
});

export const setHostUnimplemented = query(() => {
  const url = new URL("https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat");
  url.host = "qat.baz:8080";
});
