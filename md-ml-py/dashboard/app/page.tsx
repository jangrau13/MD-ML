"use client";

import { useState } from "react";
import { ActorPanel } from "./components/ActorPanel";
import { ConfigPanel } from "./components/ConfigPanel";
import { ProtocolGraph } from "./components/ProtocolGraph";
import { ReferencePanel } from "./components/ReferencePanel";
import { MatrixModal } from "./components/MatrixModal";
import { VariableBoard } from "./components/VariableBoard";
import { Math as Tex } from "./components/KaTeX";
import { useActor } from "./components/useActor";
import { HexModeProvider, Num } from "./components/NumContext";
import { useCoordinator } from "./hooks/useCoordinator";
import {
  PARTY0_URL, PARTY1_URL, DP_URLS,
  NUM_DATA_PARTIES, DATA_PARTY_CONFIGS, COMP_PARTY_CONFIGS,
  columnsForDataParty,
} from "./lib/constants";

export default function Home() {
  const party0 = useActor(PARTY0_URL);
  const party1 = useActor(PARTY1_URL);
  const dp0 = useActor(DP_URLS[0]);
  const dp1 = useActor(DP_URLS[1]);
  const dp2 = useActor(DP_URLS[2]);
  const dp3 = useActor(DP_URLS[3]);
  const dpActors = [dp0, dp1, dp2, dp3];

  const [hexMode, setHexMode] = useState(false);
  const [visiblePanels, setVisiblePanels] = useState<[number, number]>([0, 1]);
  const [resultModal, setResultModal] = useState<{
    title: string;
    dim: number;
    values: string[] | number[];
    isFloat?: boolean;
  } | null>(null);

  const { state: coord, actions } = useCoordinator(dpActors);

  // MPC result from computation parties — convert fixed-point to float
  const mpcResult: string[] | null = (() => {
    const r = party0.state?.result;
    if (!r) return null;
    try { return JSON.parse(r); } catch { return null; }
  })();
  const mpcResultFloat: number[] | null = (() => {
    if (!mpcResult || !coord.activeSession) return null;
    const d = coord.activeSession.d;
    const k = coord.configRef.current.k;
    const scale = 2 ** d;
    const mod = BigInt(1) << BigInt(k);
    const half = mod >> BigInt(1);
    return mpcResult.map((v) => {
      let n = BigInt(v);
      // Interpret as signed: if n >= 2^(k-1), n -= 2^k
      if (n >= half) n -= mod;
      return Number(n) / scale;
    });
  })();

  // Computation party readiness
  const p0s = party0.state?.current_step ?? 0;
  const p1s = party1.state?.current_step ?? 0;
  const bothDone = party0.state?.phase === "done" && party1.state?.phase === "done";
  // Cross-actor dependency helpers
  function partyReadiness(thisStep: number, otherStep: number, label: string) {
    // Step 9 (wait for Δ): blocked until data parties are done and deltas sent
    if (thisStep === 9 && !coord.sessionRef.current.allDpDone)
      return { canStep: false, reason: "Waiting for data parties" };
    // Step 8: connect — other party must also be at step >= 8
    if (thisStep === 8 && otherStep < 8)
      return { canStep: false, reason: `${label} step ${otherStep}/8` };
    // Step 20: exchange Δz' — other party must be at step >= 20
    if (thisStep === 20 && otherStep < 20)
      return { canStep: false, reason: `${label} step ${otherStep}/20` };
    // Step 23: exchange λz — other party must be at step >= 23
    if (thisStep === 23 && otherStep < 23)
      return { canStep: false, reason: `${label} step ${otherStep}/23` };
    return { canStep: true, reason: "" };
  }

  function dpReadiness(dpId: number) {
    const cols = columnsForDataParty(dpId, coord.configRef.current.dim);
    if (cols.length === 0) return { canStep: false, reason: "Inactive" };
    const dp = dpActors[dpId].state;
    if (!dp) return { canStep: false, reason: "Loading" };
    if (dp.phase === "done") return { canStep: false, reason: "" };
    // Allow stepping from idle when configured
    if (dp.phase === "idle" && !dp.configured) return { canStep: false, reason: "" };
    if (dp.current_step === 2) {
      if (p0s < 5) return { canStep: false, reason: `Party 0 step ${p0s}/5` };
      if (p1s < 5) return { canStep: false, reason: `Party 1 step ${p1s}/5` };
    }
    return { canStep: true, reason: "" };
  }

  const p0Ready = partyReadiness(p0s, p1s, "Party 1");
  const p1Ready = partyReadiness(p1s, p0s, "Party 0");

  // All panels for tab bar (only show DPs that have columns for current dim)
  const activeDpConfigs = DATA_PARTY_CONFIGS.filter((dp) =>
    columnsForDataParty(dp.id, coord.configRef.current.dim).length > 0
  );
  const allPanels = [
    ...COMP_PARTY_CONFIGS.map((a) => ({ name: a.name, icon: a.icon, color: a.color, type: "computation" as const, dpId: -1 })),
    ...activeDpConfigs.map((dp) => ({ name: dp.name, icon: dp.icon, color: dp.color, type: "data" as const, dpId: dp.id })),
  ];

  return (
    <HexModeProvider value={hexMode}>
    <div className="flex flex-col h-screen overflow-hidden">
      {/* Header */}
      <header className="border-b border-card-border bg-card-bg px-4 py-2 flex-shrink-0">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-base font-bold text-accent-blue">
              MD-ML: Secure Matrix Multiplication
            </h1>
            <p className="text-[10px] text-muted">
              <Tex tex="\pi_{\text{MultTrunc}}" /> with{" "}
              <Tex tex="\mathcal{F}_{\text{edaBits}}" /> &mdash; Fixed-Point
              Arithmetic over{" "}
              <Tex tex="\text{SPD}\mathbb{Z}_{2^k}" />
              {" "}&mdash; {NUM_DATA_PARTIES} data parties, 2 computation parties
            </p>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={() => setHexMode(!hexMode)}
              className="text-[10px] border border-card-border rounded px-2 py-0.5 cursor-pointer hover:bg-card-bg"
            >
              {hexMode ? "hex" : "dec"}
            </button>
            <ReferencePanel
              sessions={coord.sessions}
              activeSessionId={coord.activeSessionId}
              onSelectSession={actions.setActiveSessionId}
            />
          </div>
        </div>
      </header>

      {/* Config bar + graph */}
      <div className="border-b border-card-border bg-background px-4 py-2 flex-shrink-0 flex gap-4 items-start">
        <div className="flex-1 min-w-0">
          <ConfigPanel
            onConfigure={actions.handleConfigure}
            onReset={actions.handleReset}
            configured={coord.configured}
            status={coord.configStatus}
            activeSession={coord.activeSession}
          />
        </div>
        <div className="w-[420px] flex-shrink-0">
          <ProtocolGraph
            party0={party0.state}
            party1={party1.state}
            dpActors={dpActors}
            dim={coord.configRef.current.dim}
          />
        </div>
      </div>

      {/* Variable board */}
      <VariableBoard
        session={coord.activeSession}
        party0={party0.state}
        party1={party1.state}
        k={coord.configRef.current.k}
      />

      {/* Results comparison bar */}
      {coord.activeSession && (
        <div className="border-b border-card-border bg-card-bg px-4 py-2 flex-shrink-0 space-y-1">
          <div className="flex items-center gap-4 text-[11px]">
            <div className="flex-1 flex items-center gap-1 flex-wrap">
              <span className="font-semibold text-accent-purple">Plaintext</span>
              <Tex tex="A" />
              <button onClick={() => setResultModal({ title: "A (float)", dim: coord.activeSession!.dim, values: coord.activeSession!.floatA, isFloat: true })} className="text-accent-blue cursor-pointer text-[9px]">[inspect]</button>
              <span className="text-muted">&times;</span>
              <Tex tex="B" />
              <button onClick={() => setResultModal({ title: "B (float)", dim: coord.activeSession!.dim, values: coord.activeSession!.floatB, isFloat: true })} className="text-accent-blue cursor-pointer text-[9px]">[inspect]</button>
              <span className="text-muted">=</span>
              <Tex tex="C" />:
              <span className="font-mono">
                [{coord.activeSession.floatRef.slice(0, 4).map((v, i) => (
                  <span key={i}>{i > 0 && ", "}<Num value={v} /></span>
                ))}
                {coord.activeSession.floatRef.length > 4 && ", \u2026"}]
              </span>
              <button onClick={() => setResultModal({ title: "C = A \u00D7 B (float)", dim: coord.activeSession!.dim, values: coord.activeSession!.floatRef, isFloat: true })} className="text-accent-blue cursor-pointer text-[9px]">[inspect]</button>
            </div>
            <div className="flex-1">
              {mpcResultFloat ? (
                <>
                  <span className="font-semibold text-accent-green">MPC result:</span>{" "}
                  <span className="font-mono">
                    [{mpcResultFloat.slice(0, 3).map((v, i) => (
                      <span key={i}>{i > 0 && ", "}{v}</span>
                    ))}{mpcResultFloat.length > 3 && ", \u2026"}]
                  </span>
                  <button onClick={() => setResultModal({ title: "MPC Result (float)", dim: coord.activeSession!.dim, values: mpcResultFloat!, isFloat: true })} className="ml-1 text-accent-blue cursor-pointer text-[9px]">[inspect]</button>
                </>
              ) : (
                <span className="text-muted italic">MPC result: waiting for protocol to complete...</span>
              )}
            </div>
          </div>
          <div className="flex items-center gap-2 text-[10px] text-muted">
            <span>Columns:</span>
            {DATA_PARTY_CONFIGS.map((dp) => {
              const cols = columnsForDataParty(dp.id, coord.activeSession!.dim);
              if (cols.length === 0) return null;
              const isDone = dpActors[dp.id].state?.phase === "done";
              return (
                <span key={dp.id} className="font-mono" style={{ color: isDone ? dp.color : undefined }}>
                  DP{dp.id}[{cols.join(",")}]{isDone ? "\u2713" : ""}
                </span>
              );
            })}
          </div>
        </div>
      )}

      {/* Panel tabs (selection only, no auto-run) */}
      <div className="border-b border-card-border bg-card-bg px-4 py-1 flex-shrink-0 flex gap-1 flex-wrap items-center">
        {allPanels.map((p, i) => {
          const selected = visiblePanels.includes(i);
          const isComp = p.type === "computation";
          const isDone = isComp ? bothDone : coord.allDpsDone;
          return (
            <button
              key={p.name}
              onClick={() => {
                if (!selected) setVisiblePanels(([, s]) => [s, i]);
              }}
              className={`px-3 py-1 rounded text-[11px] font-semibold cursor-pointer transition-colors ${
                selected ? "text-white"
                : isDone ? "bg-accent-green/20 text-accent-green border border-accent-green/30"
                : "bg-background border border-card-border text-muted hover:text-foreground"
              }`}
              style={selected ? { backgroundColor: p.color } : undefined}
            >
              {p.icon} {p.name}
              {isDone && !selected && " \u2713"}
            </button>
          );
        })}
      </div>
      <main className="flex-1 grid grid-cols-2 gap-0 min-h-0">
        {visiblePanels.map((idx) => {
          const panel = allPanels[idx];
          if (!panel) return null;
          return (
            <div key={panel.name} className="border-r border-card-border last:border-r-0 flex flex-col min-h-0 overflow-hidden">
              {panel.type === "computation" ? (
                <ActorPanel
                  config={COMP_PARTY_CONFIGS[idx]}
                  canStepOverride={idx === 0 ? p0Ready.canStep : p1Ready.canStep}
                  blockReason={idx === 0 ? p0Ready.reason : p1Ready.reason}
                  onStepAll={actions.stepBothParties}
                />
              ) : (
                <ActorPanel
                  config={{ name: panel.name, url: DP_URLS[panel.dpId], color: panel.color, icon: panel.icon }}
                  canStepOverride={dpReadiness(panel.dpId).canStep}
                  blockReason={dpReadiness(panel.dpId).reason}
                  onStepAll={actions.stepAllDataParties}
                />
              )}
            </div>
          );
        })}
      </main>

      {resultModal && (
        <MatrixModal
          title={resultModal.title}
          dim={resultModal.dim}
          values={resultModal.values}
          isFloat={resultModal.isFloat}
          onClose={() => setResultModal(null)}
        />
      )}
    </div>
    </HexModeProvider>
  );
}
