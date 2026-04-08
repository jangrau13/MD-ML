"use client";

import { useState } from "react";
import { createPortal } from "react-dom";
import { Num, formatNum, useHexMode } from "./NumContext";
import { MathText } from "./MathText";

interface ArrayValue {
  type: "array";
  dtype?: string;
  shape?: number[];
  size: number;
  first_8?: string[];
  last_4?: string[];
}

type Value = string | number | string[] | ArrayValue;

// ── Unified value modal ────────────────────────────────────────────

function ValueModal({
  name,
  value,
  onClose,
}: {
  name: string;
  value: Value;
  onClose: () => void;
}) {
  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      onClick={onClose}
    >
      <div
        className="bg-white border border-card-border rounded-lg shadow-2xl max-w-3xl w-full mx-4 max-h-[80vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-4 py-3 border-b border-card-border">
          <h3 className="font-semibold text-sm text-accent-purple">{name}</h3>
          <button
            onClick={onClose}
            className="text-muted hover:text-foreground text-lg cursor-pointer"
          >
            ✕
          </button>
        </div>
        <div className="flex-1 overflow-auto p-4 font-mono text-xs">
          <FullValueDisplay value={value} />
        </div>
      </div>
    </div>,
    document.body
  );
}

function FullValueDisplay({ value }: { value: Value }) {
  if (value === null || value === undefined)
    return <span className="text-muted">null</span>;

  if (typeof value === "string") {
    if (/^-?\d+$/.test(value)) {
      return <div className="break-all text-accent-cyan"><Num value={value} /></div>;
    }
    return <div className="break-all text-accent-cyan"><MathText>{value}</MathText></div>;
  }

  if (typeof value === "number") {
    return <span className="text-accent-yellow"><Num value={value} /></span>;
  }

  if (Array.isArray(value)) {
    return (
      <div className="space-y-0.5">
        <div className="text-muted mb-1">{value.length} elements:</div>
        {value.map((v, i) => (
          <div key={i} className="flex gap-2">
            <span className="text-muted w-10 text-right flex-shrink-0">[{i}]</span>
            <span className="text-accent-cyan break-all"><Num value={v} /></span>
          </div>
        ))}
      </div>
    );
  }

  if (typeof value === "object" && value.type === "array") {
    const a = value as ArrayValue;
    return (
      <div>
        <div className="text-muted mb-2">
          {a.dtype && <span className="text-accent-yellow">{a.dtype} </span>}
          {a.shape && <span>shape=[{a.shape.join("x")}] </span>}
          ({a.size.toLocaleString()} elements)
        </div>
        {a.first_8 && (
          <div className="mb-2">
            <div className="text-muted mb-0.5">First 8:</div>
            {a.first_8.map((v, i) => (
              <div key={i} className="flex gap-2 ml-2">
                <span className="text-muted w-10 text-right flex-shrink-0">[{i}]</span>
                <span className="text-accent-cyan break-all"><Num value={v} /></span>
              </div>
            ))}
          </div>
        )}
        {a.last_4 && (
          <div>
            <div className="text-muted mb-0.5">Last 4 (indices {a.size - 4} to {a.size - 1}):</div>
            {a.last_4.map((v, i) => (
              <div key={i} className="flex gap-2 ml-2">
                <span className="text-muted w-10 text-right flex-shrink-0">[{a.size - 4 + i}]</span>
                <span className="text-accent-cyan break-all"><Num value={v} /></span>
              </div>
            ))}
          </div>
        )}
      </div>
    );
  }

  return <span className="text-accent-cyan whitespace-pre-wrap">{JSON.stringify(value, null, 2)}</span>;
}

// ── Inline value row (click to expand in modal) ─────────────────────

function InlineValue({ name, value }: { name: string; value: Value }) {
  const [modalOpen, setModalOpen] = useState(false);
  const hex = useHexMode();

  const summary = valueSummary(value, hex);
  const isExpandable =
    typeof value === "object" ||
    (typeof value === "string" && value.length > 30) ||
    Array.isArray(value);

  return (
    <>
      <div
        className={`flex items-start gap-1 py-0.5 ${
          isExpandable
            ? "cursor-pointer hover:bg-card-bg rounded px-1 -mx-1"
            : ""
        }`}
        onClick={isExpandable ? () => setModalOpen(true) : undefined}
      >
        <span className="text-accent-purple font-semibold flex-shrink-0">
          {name}
        </span>
        <span className="text-muted flex-shrink-0"> = </span>
        <span className="text-accent-cyan">{summary}</span>
        {isExpandable && (
          <span className="text-muted text-[9px] flex-shrink-0 ml-1">
            [click to inspect]
          </span>
        )}
      </div>
      {modalOpen && (
        <ValueModal
          name={name}
          value={value}
          onClose={() => setModalOpen(false)}
        />
      )}
    </>
  );
}

function valueSummary(val: Value, hex: boolean): string {
  if (val === null || val === undefined) return "null";
  if (typeof val === "number") return formatNum(val, hex);
  if (typeof val === "string") return formatNum(val, hex);

  if (Array.isArray(val)) {
    const fmt = (v: string) => formatNum(v, hex);
    if (val.length <= 3) return `[${val.map(fmt).join(", ")}]`;
    return `[${fmt(val[0])}, ${fmt(val[1])}, ${fmt(val[2])}, ...] (${val.length} elements)`;
  }

  if (typeof val === "object" && (val as ArrayValue).type === "array") {
    const a = val as ArrayValue;
    const first = a.first_8?.[0] ?? "?";
    return `[${formatNum(first, hex)}, ...] (${a.size.toLocaleString()} elements)`;
  }

  return JSON.stringify(val);
}

// ── Main component ──────────────────────────────────────────────────

export function ValueInspector({
  values,
}: {
  values: Record<string, Value>;
}) {
  return (
    <div className="mt-1 rounded border border-card-border bg-background/50 px-2 py-1 font-mono text-[11px] leading-relaxed">
      {Object.entries(values).map(([key, val]) => (
        <InlineValue key={key} name={key} value={val} />
      ))}
    </div>
  );
}
