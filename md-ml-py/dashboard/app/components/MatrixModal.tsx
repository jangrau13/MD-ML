"use client";

import { createPortal } from "react-dom";
import { Math as Tex } from "./KaTeX";
import { Num } from "./NumContext";

interface MatrixModalProps {
  title: string;
  dim: number;
  values: string[] | number[];
  isFloat?: boolean;
  onClose: () => void;
}

/**
 * Single unified modal for displaying matrix values.
 * Shows a grid for small matrices, a scrollable list for large ones.
 * All values shown in full — no truncation.
 */
export function MatrixModal({
  title,
  dim,
  values,
  isFloat = false,
  onClose,
}: MatrixModalProps) {
  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      onClick={onClose}
    >
      <div
        className="bg-white border border-card-border rounded-lg shadow-2xl max-w-[90vw] max-h-[85vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-card-border">
          <h3 className="font-semibold text-sm">
            {title}{" "}
            <span className="text-muted font-normal">
              — {dim}×{dim} ({values.length} elements)
            </span>
          </h3>
          <button
            onClick={onClose}
            className="text-muted hover:text-foreground text-lg cursor-pointer ml-4"
          >
            ✕
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-auto p-5">
          <table className="border-collapse font-mono text-xs">
            <thead>
              <tr>
                <th className="p-1 text-muted text-[10px] sticky left-0 bg-white"></th>
                {Array.from({ length: dim }, (_, j) => (
                  <th
                    key={j}
                    className="p-1 text-muted text-[10px] text-center sticky top-0 bg-white"
                  >
                    {j}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {Array.from({ length: dim }, (_, i) => (
                <tr key={i}>
                  <td className="p-1 text-muted text-[10px] text-right pr-2 sticky left-0 bg-white">
                    {i}
                  </td>
                  {Array.from({ length: dim }, (_, j) => (
                      <td
                        key={j}
                        className="border border-card-border px-2 py-1 text-right whitespace-nowrap"
                      >
                        <Num value={values[i * dim + j]} />
                      </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>,
    document.body
  );
}
