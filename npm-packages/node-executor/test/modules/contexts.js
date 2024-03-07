export default {
  isAction: true,

  invokeAction: async () => {
    const resp = await fetch("data:text/plain,{}");
    const fetchJson = await resp.json();
    const results = {
      "object literal instanceof Object": {} instanceof Object,
      "array literal instanceof Object": [] instanceof Array,
      "json from fetch is instanceof Object": fetchJson instanceof Object,
      "JSON.parse object is instanceof Object":
        JSON.parse("{}") instanceof Object,
      "WebAssembly.Memory intanceof SharedArrayBuffer":
        new WebAssembly.Memory({
          initial: 10,
          maximum: 100,
          shared: !0,
        }).buffer instanceof SharedArrayBuffer,
    };
    return JSON.stringify(results);
  },
};
