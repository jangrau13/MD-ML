"use client";

import { useEffect, useRef, useState } from "react";
import { LogEntry } from "./types";
import { MathText } from "./MathText";
import { LogModal } from "./LogModal";

interface LogViewerProps {
  entries: LogEntry[];
  maxHeight?: string;
}

const levelColors: Record<string, string> = {
  info: "text-foreground",
  crypto: "text-accent-cyan",
  warn: "text-accent-yellow",
  error: "text-accent-red",
};

export function LogViewer({ entries, maxHeight = "500px" }: LogViewerProps) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const prevLen = useRef(0);
  const [selectedEntry, setSelectedEntry] = useState<LogEntry | null>(null);

  useEffect(() => {
    if (entries.length > prevLen.current) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
      prevLen.current = entries.length;
    }
  }, [entries.length]);

  return (
    <>
      <div className="overflow-y-auto" style={{ maxHeight }}>
        {entries.length === 0 && (
          <div className="text-muted text-xs italic">No events yet</div>
        )}
        {entries.map((e, i) => {
          const hasValues = e.values && Object.keys(e.values).length > 0;
          return (
            <div
              key={i}
              className={`py-1 border-b border-card-border last:border-0 ${
                hasValues
                  ? "cursor-pointer hover:bg-card-bg rounded px-1 -mx-1"
                  : ""
              }`}
              onClick={hasValues ? () => setSelectedEntry(e) : undefined}
            >
              <div className="font-mono text-[11px] flex items-start gap-1">
                <span className="text-muted flex-shrink-0">
                  [{e.time.toFixed(3)}s]
                </span>
                <span className={levelColors[e.level] ?? "text-foreground"}>
                  <MathText>{e.msg}</MathText>
                </span>
                {hasValues && (
                  <span className="text-accent-blue text-[9px] flex-shrink-0 ml-auto">
                    [inspect]
                  </span>
                )}
              </div>
            </div>
          );
        })}
        <div ref={bottomRef} />
      </div>

      {selectedEntry && (
        <LogModal
          entry={selectedEntry}
          onClose={() => setSelectedEntry(null)}
        />
      )}
    </>
  );
}
