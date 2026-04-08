"use client";

import { useState } from "react";
import { Math as Tex } from "./KaTeX";
import { Session } from "./ReferencePanel";
import { ActorState } from "./types";
import { Num } from "./NumContext";

interface VariableBoardProps {
  session: Session | null;
  party0: ActorState | null;
  party1: ActorState | null;
  k: number;
}

interface VarEntry {
  symbol: string;
  name: string;
  value: string;
  category: "params" | "keys" | "data" | "result";
}

export function VariableBoard({
  session,
  party0,
  party1,
  k: kProp,
}: VariableBoardProps) {
  const [open, setOpen] = useState(true);

  if (!session) return null;

  const d = session.d;
  const k = kProp;
  const s = 64;  // MP-SPDZ enforces s=64 security parameter
  const n = session.dim;

  const vars: VarEntry[] = [
    { symbol: "k", name: "Value ring bits", value: `${k}`, category: "params" },
    { symbol: "s", name: "Security parameter", value: `${s}`, category: "params" },
    { symbol: "k+s", name: "Combined ring bits", value: `${k + s}`, category: "params" },
    { symbol: "d", name: "Fractional bits (FPA)", value: `${d}`, category: "params" },
    { symbol: "2^d", name: "Fixed-point scale factor", value: `${(2 ** d).toLocaleString()}`, category: "params" },
    { symbol: "n", name: "Matrix dimension", value: `${n}`, category: "params" },
    { symbol: "n^2", name: "Elements per matrix", value: `${(n * n).toLocaleString()}`, category: "params" },
    { symbol: "\\mathbb{Z}_{2^k}", name: "Value ring", value: `integers mod 2^${k}`, category: "params" },
    { symbol: "\\mathbb{Z}_{2^{k+s}}", name: "Computation ring", value: `integers mod 2^${k + s}`, category: "params" },
  ];

  // Extract MAC key shares from party logs
  for (const [label, partyState] of [["0", party0], ["1", party1]] as const) {
    const log = (partyState as ActorState | null)?.log ?? [];
    for (const entry of log) {
      if (entry.values) {
        const v = entry.values as Record<string, unknown>;
        const key = `α^${label}` in v ? `α^${label}` : "α^i" in v ? "α^i" : null;
        if (key) {
          vars.push({
            symbol: `\\alpha^${label}`,
            name: `Party ${label} MAC key share`,
            value: String(v[key]),
            category: "keys",
          });
        }
      }
    }
  }

  // Communication cost
  const p0Sent = party0?.bytes_sent ?? 0;
  const p1Sent = party1?.bytes_sent ?? 0;
  if (p0Sent > 0 || p1Sent > 0) {
    vars.push({ symbol: "\\text{comm}_0", name: "Party 0 bytes sent", value: `${p0Sent.toLocaleString()} B`, category: "result" });
    vars.push({ symbol: "\\text{comm}_1", name: "Party 1 bytes sent", value: `${p1Sent.toLocaleString()} B`, category: "result" });
  }

  const categories = [
    { key: "params" as const, label: "Protocol Parameters" },
    { key: "keys" as const, label: "MAC Keys" },
    { key: "result" as const, label: "Communication" },
  ];

  return (
    <div className="border-b border-card-border bg-card-bg flex-shrink-0">
      <button
        onClick={() => setOpen(!open)}
        className="w-full px-4 py-1 text-[10px] text-muted hover:text-foreground cursor-pointer flex items-center gap-1"
      >
        <span>{open ? "▼" : "▶"}</span>
        <span>Variable Board</span>
      </button>
      {open && (
        <div className="px-4 pb-2 grid grid-cols-3 gap-x-6 gap-y-0">
          {categories.map((cat) => {
            const catVars = vars.filter((v) => v.category === cat.key);
            if (catVars.length === 0) return null;
            return (
              <div key={cat.key}>
                <div className="text-[9px] text-muted font-semibold uppercase tracking-wider mb-0.5">
                  {cat.label}
                </div>
                {catVars.map((v, i) => (
                  <div
                    key={i}
                    className="flex items-baseline gap-2 text-[11px] py-0.5 border-b border-card-border last:border-0"
                  >
                    <span className="flex-shrink-0 w-20 text-right">
                      <Tex tex={v.symbol} />
                    </span>
                    <span className="text-muted text-[10px] flex-1 truncate">
                      {v.name}
                    </span>
                    <span className="font-mono text-accent-blue font-semibold flex-shrink-0">
                      <Num value={v.value} />
                    </span>
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
