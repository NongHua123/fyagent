const normalizeHostname = (hostname: string): string =>
  hostname
    .trim()
    .toLowerCase()
    .replace(/^\[|\]$/g, "")
    .replace(/\.$/, "");

const isIpv4Loopback = (hostname: string): boolean => {
  const octets = hostname.split(".");
  if (octets.length !== 4) return false;

  const numbers = octets.map((octet) => {
    if (!/^\d{1,3}$/.test(octet)) return Number.NaN;
    return Number(octet);
  });

  return (
    numbers.every(
      (octet) => Number.isInteger(octet) && octet >= 0 && octet <= 255,
    ) && numbers[0] === 127
  );
};

const isIpv4MappedIpv6Loopback = (hostname: string): boolean => {
  // URL normalizes an IPv4-mapped literal such as ::ffff:127.0.0.1 to
  // ::ffff:7f00:1. Check the final 32 bits after that normalization too.
  const match = hostname.match(
    /^::(?:ffff:)?([0-9a-f]{1,4}):([0-9a-f]{1,4})$/i,
  );
  if (!match) return false;

  const high = Number.parseInt(match[1], 16);
  return high >= 0x7f00 && high <= 0x7fff;
};

/**
 * Browser URL parsing gives us a canonical literal hostname. This intentionally
 * recognizes only addresses which are intrinsically loopback; arbitrary DNS
 * names are treated as remote even if a particular machine happens to resolve
 * them locally.
 */
export const isWorkBuddyLoopbackHostname = (hostname: string): boolean => {
  const normalized = normalizeHostname(hostname);
  return (
    normalized === "localhost" ||
    normalized === "::1" ||
    isIpv4Loopback(normalized) ||
    isIpv4MappedIpv6Loopback(normalized)
  );
};

/**
 * HTTP remains an allowed WorkBuddy target. A non-loopback host is warned
 * about locally so the UI remains correct even though the restricted backend
 * result does not carry presentation-only warning state.
 */
export const hasWorkBuddyRemoteHttpWarning = (baseUrl: string): boolean => {
  try {
    const url = new URL(baseUrl.trim());
    return (
      url.protocol === "http:" && !isWorkBuddyLoopbackHostname(url.hostname)
    );
  } catch {
    return false;
  }
};
