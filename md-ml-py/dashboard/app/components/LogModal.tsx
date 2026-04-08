"use client";

import { createPortal } from "react-dom";
import { MathText } from "./MathText";
import { LogEntry } from "./types";
import { Num } from "./NumContext";

interface LogModalProps {
  entry: LogEntry;
  onClose: () => void;
}

interface ArrayValue {
  type: "array";
  dtype?: string;
  shape?: number[];
  size: number;
  first_8?: string[];
  last_4?: string[];
}

type Value = string | number | string[] | ArrayValue;

export function LogModal({ entry, onClose }: LogModalProps) {
  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      onClick={onClose}
    >
      <div
        className="bg-white border border-card-border rounded-lg shadow-2xl w-[95vw] h-[90vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-5 py-3 border-b border-card-border">
          <div className="flex items-center justify-between">
            <span className="text-muted text-xs font-mono">
              [{entry.time.toFixed(3)}s]
            </span>
            <button
              onClick={onClose}
              className="text-muted hover:text-foreground text-lg cursor-pointer"
            >
              ✕
            </button>
          </div>
          <h3 className="font-semibold text-base mt-1">
            <MathText>{entry.msg}</MathText>
          </h3>
        </div>

        {/* Body: values rendered with math */}
        <div className="flex-1 overflow-auto p-5 space-y-4">
          {entry.values ? (
            Object.entries(entry.values).map(([key, val]) => (
              <ValueBlock key={key} name={key} value={val as Value} />
            ))
          ) : (
            <p className="text-muted text-sm italic">
              No values attached to this log entry.
            </p>
          )}
        </div>
      </div>
    </div>,
    document.body
  );
}

function ValueBlock({ name, value }: { name: string; value: Value }) {
  return (
    <div className="border border-card-border rounded-lg overflow-hidden">
      {/* Variable name as math */}
      <div className="bg-card-bg px-4 py-2 border-b border-card-border">
        <span className="font-semibold text-sm">
          <MathText>{name}</MathText>
        </span>
      </div>
      {/* Value display */}
      <div className="p-4 font-mono text-xs overflow-auto">
        <RenderValue value={value} />
      </div>
    </div>
  );
}

function RenderValue({ value }: { value: Value }) {
  if (value === null || value === undefined) {
    return <span className="text-muted">null</span>;
  }

  if (typeof value === "boolean") {
    return (
      <span className={value ? "text-accent-green font-bold" : "text-accent-red font-bold"}>
        {String(value)}
      </span>
    );
  }

  if (typeof value === "number") {
    return <span className="text-accent-blue text-lg font-semibold"><Num value={value} /></span>;
  }

  if (typeof value === "string") {
    // Numeric strings → Num (supports hex/dec toggle), everything else → MathText
    if (/^-?\d+$/.test(value)) {
      return <span className="text-accent-blue text-base"><Num value={value} /></span>;
    }
    return <span className="text-accent-blue text-base"><MathText>{value}</MathText></span>;
  }

  if (Array.isArray(value)) {
    // Flat array — show as numbered list
    return (
      <div>
        <div className="text-muted mb-2">{value.length} elements</div>
        <div className="grid gap-0.5" style={{ gridTemplateColumns: "auto 1fr" }}>
          {value.map((v, i) => (
            <div key={i} className="contents">
              <span className="text-muted text-right pr-3">[{i}]</span>
              <span className="text-accent-blue"><Num value={v} /></span>
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (typeof value === "object" && (value as ArrayValue).type === "array") {
    const a = value as ArrayValue;
    return (
      <div>
        <div className="text-muted mb-2">
          {a.dtype && <span>{a.dtype} </span>}
          {a.shape && <span>shape=[{a.shape.join(" x ")}] </span>}
          — {a.size.toLocaleString()} elements
        </div>
        {a.first_8 && (
          <div className="mb-3">
            <div className="text-muted text-[10px] mb-1 font-semibold">First 8 values:</div>
            <div className="grid gap-0.5" style={{ gridTemplateColumns: "auto 1fr" }}>
              {a.first_8.map((v, i) => (
                <div key={i} className="contents">
                  <span className="text-muted text-right pr-3">[{i}]</span>
                  <span className="text-accent-blue"><Num value={v} /></span>
                </div>
              ))}
            </div>
          </div>
        )}
        {a.last_4 && (
          <div>
            <div className="text-muted text-[10px] mb-1 font-semibold">
              Last 4 values (indices {a.size - 4} to {a.size - 1}):
            </div>
            <div className="grid gap-0.5" style={{ gridTemplateColumns: "auto 1fr" }}>
              {a.last_4.map((v, i) => (
                <div key={i} className="contents">
                  <span className="text-muted text-right pr-3">[{a.size - 4 + i}]</span>
                  <span className="text-accent-blue"><Num value={v} /></span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    );
  }

  // Fallback for unknown objects
  return (
    <pre className="text-accent-blue whitespace-pre-wrap text-xs">
      {JSON.stringify(value, null, 2)}
    </pre>
  );
}
