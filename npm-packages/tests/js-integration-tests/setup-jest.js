import dns from "node:dns";

/**
 * Node defaults to ipv6, and since usher is running locally with ipv4 addresses,
 * set the default result order to ipv4
 */
export const setDnsToIpv4First = () => {
  dns.setDefaultResultOrder("ipv4first");
};

beforeAll(setDnsToIpv4First);
