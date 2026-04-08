import { ActorConfig } from "../components/types";

export const PARTY0_URL =
  process.env.NEXT_PUBLIC_PARTY0_URL ?? "http://localhost:8081";
export const PARTY1_URL =
  process.env.NEXT_PUBLIC_PARTY1_URL ?? "http://localhost:8082";

export const NUM_DATA_PARTIES = 4;
export const DP_URLS = [
  process.env.NEXT_PUBLIC_DP0_URL ?? "http://localhost:8090",
  process.env.NEXT_PUBLIC_DP1_URL ?? "http://localhost:8091",
  process.env.NEXT_PUBLIC_DP2_URL ?? "http://localhost:8092",
  process.env.NEXT_PUBLIC_DP3_URL ?? "http://localhost:8093",
];

export const DATA_PARTY_CONFIGS = [
  { id: 0, name: "DP 0", url: DP_URLS[0], color: "#d29922", icon: "🟡" },
  { id: 1, name: "DP 1", url: DP_URLS[1], color: "#a371f7", icon: "🟣" },
  { id: 2, name: "DP 2", url: DP_URLS[2], color: "#f47067", icon: "🔴" },
  { id: 3, name: "DP 3", url: DP_URLS[3], color: "#57ab5a", icon: "🟢" },
];

export const COMP_PARTY_CONFIGS: ActorConfig[] = [
  { name: "Party 0", url: PARTY0_URL, color: "#0969da", icon: "🔵" },
  { name: "Party 1", url: PARTY1_URL, color: "#1a7f37", icon: "🟢" },
];

export function makeMask(k: number): bigint {
  return (BigInt(1) << BigInt(k)) - BigInt(1);
}

/** Return which column indices data party `dpId` owns for a given dim. */
export function columnsForDataParty(dpId: number, dim: number): number[] {
  if (dpId >= dim) return [];
  const cols: number[] = [];
  for (let c = dpId; c < dim; c += NUM_DATA_PARTIES) {
    cols.push(c);
  }
  return cols;
}
