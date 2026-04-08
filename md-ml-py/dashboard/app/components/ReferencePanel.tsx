"use client";

import { useState } from "react";
import { createPortal } from "react-dom";
import { Math as Tex } from "./KaTeX";
import { Num } from "./NumContext";

interface Session {
  id: number;
  dim: number;
  d: number;
  floatA: number[];
  floatB: number[];
  floatRef: number[];
  matA: string[];
  matB: string[];
  refResult: string[];
  timestamp: number;
}

interface ReferencePanelProps {
  sessions: Session[];
  activeSessionId: number | null;
  onSelectSession: (id: number) => void;
}

export function ReferencePanel({
  sessions,
  activeSessionId,
  onSelectSession,
}: ReferencePanelProps) {
  const [expanded, setExpanded] = useState<"A" | "B" | "C" | null>(null);
  const active = sessions.find((s) => s.id === activeSessionId);

  return (
    <div className="flex flex-col gap-2">
      {/* Session picker */}
      <div className="flex items-center gap-2">
        <span className="text-[10px] text-muted">Session:</span>
        {sessions.length === 0 ? (
          <span className="text-[10px] text-muted italic">
            No sessions yet — configure a computation
          </span>
        ) : (
          <select
            value={activeSessionId ?? ""}
            onChange={(e) => onSelectSession(Number(e.target.value))}
            className="bg-background border border-card-border rounded px-2 py-0.5 text-[11px] font-mono"
          >
            {sessions.map((s) => (
              <option key={s.id} value={s.id}>
                #{s.id} — {s.dim}×{s.dim} (
                {new Date(s.timestamp).toLocaleTimeString()})
              </option>
            ))}
          </select>
        )}
      </div>

      {/* Reference values */}
      {active && (
        <div className="flex gap-2 items-start">
          <MatrixBadge
            label="A"
            tex="A"
            dim={active.dim}
            values={active.matA}
            color="#58a6ff"
            expanded={expanded === "A"}
            onToggle={() => setExpanded(expanded === "A" ? null : "A")}
          />
          <span className="text-muted self-center text-sm">×</span>
          <MatrixBadge
            label="B"
            tex="B"
            dim={active.dim}
            values={active.matB}
            color="#3fb950"
            expanded={expanded === "B"}
            onToggle={() => setExpanded(expanded === "B" ? null : "B")}
          />
          <span className="text-muted self-center text-sm">=</span>
          <MatrixBadge
            label="C"
            tex="A \times B"
            dim={active.dim}
            values={active.refResult}
            color="#d29922"
            expanded={expanded === "C"}
            onToggle={() => setExpanded(expanded === "C" ? null : "C")}
          />
        </div>
      )}
    </div>
  );
}

function MatrixBadge({
  label,
  tex,
  dim,
  values,
  color,
  expanded,
  onToggle,
}: {
  label: string;
  tex: string;
  dim: number;
  values: string[];
  color: string;
  expanded: boolean;
  onToggle: () => void;
}) {
  const [modalOpen, setModalOpen] = useState(false);

  return (
    <>
      <button
        onClick={onToggle}
        className="rounded border border-card-border bg-card-bg px-2 py-1 text-[10px] font-mono hover:bg-background/50 transition-colors cursor-pointer flex items-center gap-1.5"
        style={{ borderColor: color + "44" }}
      >
        <Tex tex={tex} />
        <span className="text-muted">
          {dim}×{dim}
        </span>
        <span
          className="ml-1 cursor-pointer"
          onClick={(e) => {
            e.stopPropagation();
            setModalOpen(true);
          }}
        >
          🔍
        </span>
      </button>

      {/* Inline preview */}
      {expanded && (
        <div className="absolute z-10 mt-8 rounded border border-card-border bg-card-bg shadow-xl p-2 font-mono text-[10px] max-w-xs overflow-auto max-h-48">
          <div className="text-muted mb-1">
            <Tex tex={tex} /> ({dim}×{dim}, first 8 values):
          </div>
          {values.slice(0, 8).map((v, i) => (
            <div key={i} className="flex gap-1">
              <span className="text-muted w-6 text-right">[{i}]</span>
              <Num value={v} />
            </div>
          ))}
          {values.length > 8 && (
            <div className="text-muted mt-1">
              … {values.length - 8} more
            </div>
          )}
        </div>
      )}

      {/* Full modal */}
      {modalOpen &&
        createPortal(
          <div
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
            onClick={() => setModalOpen(false)}
          >
            <div
              className="bg-card-bg border border-card-border rounded-lg shadow-2xl max-w-3xl w-full mx-4 max-h-[80vh] flex flex-col"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="flex items-center justify-between px-4 py-3 border-b border-card-border">
                <h3 className="font-semibold text-sm" style={{ color }}>
                  <Tex tex={tex} /> — {dim}×{dim} (
                  {values.length.toLocaleString()} elements)
                </h3>
                <button
                  onClick={() => setModalOpen(false)}
                  className="text-muted hover:text-foreground text-lg cursor-pointer"
                >
                  ✕
                </button>
              </div>
              <div className="flex-1 overflow-auto p-4">
                {/* Show as grid for small matrices */}
                {dim <= 8 ? (
                  <table className="font-mono text-[10px] border-collapse">
                    <tbody>
                      {Array.from({ length: dim }, (_, row) => (
                        <tr key={row}>
                          {Array.from({ length: dim }, (_, col) => (
                            <td
                              key={col}
                              className="border border-card-border px-1 py-0.5"
                              style={{ color }}
                            >
                              <Num value={values[row * dim + col]} />
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                ) : (
                  <div className="font-mono text-[10px] space-y-0.5">
                    {values.map((v, i) => {
                      const row = Math.floor(i / dim);
                      const col = i % dim;
                      return (
                        <div key={i} className="flex gap-2">
                          <span className="text-muted w-16 text-right flex-shrink-0">
                            [{row},{col}]
                          </span>
                          <Num value={v} />
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>
          </div>,
          document.body
        )}
    </>
  );
}


export type { Session };
