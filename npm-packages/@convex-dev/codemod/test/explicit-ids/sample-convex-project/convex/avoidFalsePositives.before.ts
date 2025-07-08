// Regression test for a false positive in IntDate.get from jsrsasign
// eslint-disable-next-line @typescript-eslint/no-namespace
declare namespace MyNamespace {
  function get(parameter: number): string;
}

async function _ignoreUnrelatedFunctionsFromNamespaces() {
  MyNamespace.get(1);
}

// Ignore methods from lib types
new URL("https://www.convex.dev?test=1").searchParams.get("test");

// Ignore .replace on string
console.log("test".replace("test", "test2"));
