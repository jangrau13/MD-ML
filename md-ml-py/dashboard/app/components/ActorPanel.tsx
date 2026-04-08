"use client";

import { useState } from "react";
import { ActorConfig } from "./types";
import { useActor } from "./useActor";
import { StepList } from "./StepList";
import { LogViewer } from "./LogViewer";

interface ActorPanelProps {
  config: ActorConfig;
  /** Override the default canStep logic (cross-actor deps). undefined = use default. */
  canStepOverride?: boolean;
  /** Reason the step is blocked (shown as tooltip / label). */
  blockReason?: string;
  /** If true, hide the individual step button (stepping is done externally via "Step Both"). */
  hideStepButton?: boolean;
  /** Callback to step all actors of the same group (e.g. all DPs or both computation parties). */
  onStepAll?: () => void;
}

function fmtBytes(n: number): string {
  if (n >= 1e9) return (n / 1e9).toFixed(2) + " GB";
  if (n >= 1e6) return (n / 1e6).toFixed(2) + " MB";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + " KB";
  return n + " B";
}

const phaseColors: Record<string, string> = {
  idle: "text-muted",
  offline: "text-accent-purple",
  preprocessing: "text-accent-purple",
  generating: "text-accent-purple",
  connecting: "text-accent-yellow",
  serving: "text-accent-yellow",
  online: "text-accent-green",
  done: "text-accent-blue",
  error: "text-accent-red",
};

export function ActorPanel({ config, canStepOverride, blockReason, hideStepButton, onStepAll }: ActorPanelProps) {
  const { state, error, isLoading, triggerStep } = useActor(config.url);
  const [tab, setTab] = useState<"steps" | "log">("steps");

  if (isLoading && !state) {
    return (
      <div className="p-3 h-full">
        <h2 className="text-sm font-semibold" style={{ color: config.color }}>
          {config.icon} {config.name}
        </h2>
        <div className="text-muted text-xs animate-pulse-glow mt-1">
          Connecting...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-3 h-full">
        <h2 className="text-sm font-semibold" style={{ color: config.color }}>
          {config.icon} {config.name}
        </h2>
        <div className="text-accent-red text-xs mt-1">Offline</div>
      </div>
    );
  }

  if (!state) return null;

  const localCanStep =
    state.configured &&
    state.phase !== "done" &&
    state.current_step < state.total_steps;

  // Cross-actor override takes precedence when it would block
  const canStep = localCanStep && (canStepOverride ?? true);

  // Short button label — just the step number
  let btnLabel = "Waiting for config...";
  if (!state.configured) {
    btnLabel = "Waiting for config...";
  } else if (state.current_step === 0 && state.phase === "idle") {
    btnLabel = "▶ Start";
  } else if (state.current_step >= state.total_steps || state.phase === "done") {
    btnLabel = "✓ Done";
  } else if (localCanStep && !canStep && blockReason) {
    btnLabel = `⏳ ${blockReason}`;
  } else {
    btnLabel = `▶ Step ${state.current_step + 1}`;
  }

  return (
    <div className="flex flex-col h-full bg-card-bg">
      {/* Compact header */}
      <div className="px-3 pt-2 pb-1 border-b border-card-border flex-shrink-0">
        <div className="flex items-center justify-between">
          <h2
            className="text-sm font-bold"
            style={{ color: config.color }}
          >
            {config.icon} {config.name}
          </h2>
          <span
            className={`text-[10px] font-mono px-1.5 py-0.5 rounded bg-background ${
              state.phase !== "idle" && state.phase !== "done"
                ? "animate-pulse-glow"
                : ""
            } ${phaseColors[state.phase] ?? "text-foreground"}`}
          >
            {state.phase}
          </span>
        </div>
        <div className="text-[10px] text-muted truncate mt-0.5">
          {state.status}
        </div>

        {/* Stats row */}
        <div className="flex gap-4 text-[10px] mt-1 mb-1">
          <div>
            <span className="text-muted">Step </span>
            <span className="font-mono font-semibold">
              {state.current_step}/{state.total_steps}
            </span>
          </div>
          <div>
            <span className="text-muted">Sent </span>
            <span className="font-mono font-semibold">
              {fmtBytes(state.bytes_sent)}
            </span>
          </div>
          <div>
            <span className="text-muted">Offline </span>
            <span className="font-mono font-semibold">
              {fmtBytes(state.bytes_received_offline)}
            </span>
          </div>
        </div>

        {/* Clickable step progress bar */}
        {(() => {
          const pct = state.total_steps > 0
            ? Math.round((state.current_step / state.total_steps) * 100)
            : 0;
          const isDone = state.phase === "done" || state.current_step >= state.total_steps;
          const stepHandler = onStepAll ?? triggerStep;
          return (
            <button
              onClick={stepHandler}
              disabled={!canStep}
              className={`w-full rounded text-xs font-semibold transition-colors relative overflow-hidden ${
                canStep
                  ? "cursor-pointer hover:brightness-110"
                  : "cursor-not-allowed"
              }`}
              style={{ height: "28px" }}
            >
              {/* Background track */}
              <div className={`absolute inset-0 ${isDone ? "bg-accent-blue/20" : "bg-card-border"}`} />
              {/* Fill bar */}
              <div
                className={`absolute inset-y-0 left-0 transition-all duration-300 ${
                  isDone ? "bg-accent-blue/40" : canStep ? "bg-accent-green/60" : "bg-card-border"
                }`}
                style={{ width: `${pct}%` }}
              />
              {/* Label */}
              <span className={`relative z-10 ${
                isDone ? "text-accent-blue" : canStep ? "text-foreground" : "text-muted"
              }`}>
                {btnLabel}
              </span>
            </button>
          );
        })()}
      </div>

      {/* Tabs */}
      <div className="flex border-b border-card-border text-[10px] flex-shrink-0">
        <button
          onClick={() => setTab("steps")}
          className={`flex-1 py-1.5 transition-colors cursor-pointer ${
            tab === "steps"
              ? "border-b-2 border-accent-blue text-accent-blue"
              : "text-muted hover:text-foreground"
          }`}
        >
          Steps ({state.current_step}/{state.total_steps})
        </button>
        <button
          onClick={() => setTab("log")}
          className={`flex-1 py-1.5 transition-colors cursor-pointer ${
            tab === "log"
              ? "border-b-2 border-accent-blue text-accent-blue"
              : "text-muted hover:text-foreground"
          }`}
        >
          Log ({state.log.length})
        </button>
      </div>

      {/* Scrollable content */}
      <div className="flex-1 overflow-y-auto p-2 min-h-0">
        {tab === "steps" ? (
          <StepList
            steps={state.steps}
            currentStep={state.current_step}
            phase={state.phase}
          />
        ) : (
          <LogViewer entries={state.log} maxHeight="none" />
        )}
      </div>
    </div>
  );
}
