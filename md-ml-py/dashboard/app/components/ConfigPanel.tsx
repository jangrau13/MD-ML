"use client";

import { useState } from "react";
import { Math as Tex } from "./KaTeX";
import { MatrixModal } from "./MatrixModal";
import { Session } from "./ReferencePanel";
import { Num } from "./NumContext";

interface ConfigPanelProps {
  onConfigure: (dim: number, d: number, k: number) => void;
  onReset: () => void;
  configured: boolean;
  status: string;
  activeSession: Session | null;
}

export function ConfigPanel({
  onConfigure,
  onReset,
  configured,
  status,
  activeSession,
}: ConfigPanelProps) {
  const [dim, setDim] = useState(4);
  const [d, setD] = useState(20);
  const [k, setK] = useState(64);
  const [modal, setModal] = useState<{
    title: string;
    dim: number;
    values: string[] | number[];
    isFloat?: boolean;
  } | null>(null);

  return (
    <>
      <div className="rounded-lg border border-card-border bg-card-bg p-3">
        {!configured ? (
          <>
            <h2 className="text-sm font-semibold text-accent-yellow mb-2">
              Configure Computation
            </h2>
            <div className="mb-2 text-xs text-muted">
              <Tex
                tex={`C = A \\times B \\bmod 2^{${k}}`}
                display
                className="mb-1 block"
              />
              <p className="mt-1">
                Fixed-point arithmetic: float values are scaled by{" "}
                <Tex tex="2^d" />, multiplied as integers, then truncated by{" "}
                <Tex tex="2^d" /> via{" "}
                <Tex tex="\pi_{\text{MultTrunc}}" />.
              </p>
            </div>
            <div className="flex items-center gap-3 flex-wrap">
              <label className="text-xs text-muted">
                <Tex tex="n" />:
              </label>
              <select
                value={dim}
                onChange={(e) => setDim(Number(e.target.value))}
                className="bg-background border border-card-border rounded px-2 py-1 text-xs"
              >
                {[2, 4, 8, 16, 32, 64].map((n) => (
                  <option key={n} value={n}>
                    {n}x{n} ({(n * n).toLocaleString()} el)
                  </option>
                ))}
              </select>
              <label className="text-xs text-muted">
                <Tex tex="d" /> (frac. bits):
              </label>
              <select
                value={d}
                onChange={(e) => setD(Number(e.target.value))}
                className="bg-background border border-card-border rounded px-2 py-1 text-xs"
              >
                {[1, 2, 4, 8, 12, 16, 20, 24].map((v) => (
                  <option key={v} value={v}>
                    {v} (scale {(2 ** v).toLocaleString()})
                  </option>
                ))}
              </select>
              <label className="text-xs text-muted">
                <Tex tex="k" /> (ring bits):
              </label>
              <select
                value={k}
                onChange={(e) => setK(Number(e.target.value))}
                className="bg-background border border-card-border rounded px-2 py-1 text-xs"
              >
                {[5, 10, 32, 64].map((v) => (
                  <option key={v} value={v}>
                    {v} (mod 2^{v})
                  </option>
                ))}
              </select>
              <button
                onClick={() => onConfigure(dim, d, k)}
                className="bg-accent-green text-white px-4 py-1.5 rounded font-semibold text-xs hover:bg-accent-green/80 transition-colors cursor-pointer"
              >
                Generate &amp; Distribute
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="flex items-center gap-3 mb-2">
              <div className="text-xs text-accent-green font-semibold">
                {status}
              </div>
              <button
                onClick={onReset}
                className="text-[10px] border border-card-border rounded px-2 py-0.5 cursor-pointer hover:bg-background text-muted hover:text-foreground transition-colors"
              >
                New Computation
              </button>
            </div>
            {activeSession && (
              <>
                {/* Inline decimal matrix calculation */}
                <div className="mb-2 overflow-x-auto">
                  <div className="flex items-center gap-2 text-[10px] font-mono">
                    {/* Matrix A */}
                    <InlineMatrix
                      label="A"
                      dim={activeSession.dim}
                      values={activeSession.floatA}
                      color="text-accent-blue"
                      onInspect={() =>
                        setModal({
                          title: "A (float)",
                          dim: activeSession.dim,
                          values: activeSession.floatA,
                          isFloat: true,
                        })
                      }
                    />
                    <span className="text-muted text-sm self-center">×</span>
                    {/* Matrix B */}
                    <InlineMatrix
                      label="B"
                      dim={activeSession.dim}
                      values={activeSession.floatB}
                      color="text-accent-green"
                      onInspect={() =>
                        setModal({
                          title: "B (float)",
                          dim: activeSession.dim,
                          values: activeSession.floatB,
                          isFloat: true,
                        })
                      }
                    />
                    <span className="text-muted text-sm self-center">=</span>
                    {/* Matrix C */}
                    <InlineMatrix
                      label="C"
                      dim={activeSession.dim}
                      values={activeSession.floatRef}
                      color="text-accent-yellow"
                      onInspect={() =>
                        setModal({
                          title: "C = A × B (float reference)",
                          dim: activeSession.dim,
                          values: activeSession.floatRef,
                          isFloat: true,
                        })
                      }
                    />
                  </div>
                </div>
                {/* Fixed-point buttons */}
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="text-[10px] text-muted">Fixed-point:</span>
                  <button
                    onClick={() =>
                      setModal({
                        title: "A (fixed-point integer)",
                        dim: activeSession.dim,
                        values: activeSession.matA,
                      })
                    }
                    className="rounded border border-card-border bg-background px-2.5 py-1 text-[11px] font-mono hover:shadow transition cursor-pointer"
                  >
                    <Tex tex="A_{\text{fixed}}" />
                  </button>
                  <button
                    onClick={() =>
                      setModal({
                        title: "B (fixed-point integer)",
                        dim: activeSession.dim,
                        values: activeSession.matB,
                      })
                    }
                    className="rounded border border-card-border bg-background px-2.5 py-1 text-[11px] font-mono hover:shadow transition cursor-pointer"
                  >
                    <Tex tex="B_{\text{fixed}}" />
                  </button>
                  <button
                    onClick={() =>
                      setModal({
                        title: "C (fixed-point reference, truncated)",
                        dim: activeSession.dim,
                        values: activeSession.refResult,
                      })
                    }
                    className="rounded border border-card-border bg-background px-2.5 py-1 text-[11px] font-mono hover:shadow transition cursor-pointer"
                  >
                    <Tex tex="C_{\text{fixed}}" />
                  </button>
                </div>
              </>
            )}

          </>
        )}
      </div>

      {modal && (
        <MatrixModal
          title={modal.title}
          dim={modal.dim}
          values={modal.values}
          isFloat={modal.isFloat}
          onClose={() => setModal(null)}
        />
      )}
    </>
  );
}

/** Renders a small inline matrix with bracket notation; clickable to open full modal. */
function InlineMatrix({
  label,
  dim,
  values,
  color,
  onInspect,
}: {
  label: string;
  dim: number;
  values: number[];
  color: string;
  onInspect: () => void;
}) {
  const MAX_DISPLAY = 4; // show up to 4×4 inline
  const showDim = Math.min(dim, MAX_DISPLAY);
  const truncated = dim > MAX_DISPLAY;

  return (
    <div className="flex flex-col items-center gap-0.5">
      <span className="text-muted text-[9px] font-semibold">{label}</span>
      <button
        onClick={onInspect}
        className="rounded border border-card-border bg-background px-1.5 py-1 hover:shadow transition cursor-pointer"
        title={`Click to inspect full ${dim}×${dim} matrix`}
      >
        <table className="border-collapse">
          <tbody>
            {Array.from({ length: showDim }, (_, row) => (
              <tr key={row}>
                {Array.from({ length: showDim }, (_, col) => (
                  <td
                    key={col}
                    className={`px-1 py-0 text-right ${color}`}
                    style={{ fontSize: "9px", lineHeight: "14px" }}
                  >
                    <Num value={values[row * dim + col]} />
                  </td>
                ))}
                {truncated && row === 0 && (
                  <td
                    rowSpan={showDim}
                    className="text-muted pl-1 align-middle"
                    style={{ fontSize: "9px" }}
                  >
                    …
                  </td>
                )}
              </tr>
            ))}
            {truncated && (
              <tr>
                {Array.from({ length: showDim }, (_, col) => (
                  <td
                    key={col}
                    className="text-muted text-center"
                    style={{ fontSize: "9px" }}
                  >
                    ⋮
                  </td>
                ))}
              </tr>
            )}
          </tbody>
        </table>
      </button>
    </div>
  );
}
