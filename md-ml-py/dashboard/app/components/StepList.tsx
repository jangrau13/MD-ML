"use client";

import { StepDef } from "./types";
import { MathText } from "./MathText";

interface StepListProps {
  steps: StepDef[];
  currentStep: number;
  phase: string;
}

/** Color palette for step phases */
const phaseColors: Record<string, { dot: string; doneDot: string; label: string; labelText: string }> = {
  offline:        { dot: "bg-accent-purple",  doneDot: "bg-accent-purple",  label: "bg-accent-purple/15", labelText: "text-accent-purple" },
  preprocessing:  { dot: "bg-accent-purple",  doneDot: "bg-accent-purple",  label: "bg-accent-purple/15", labelText: "text-accent-purple" },
  connecting:     { dot: "bg-accent-yellow",  doneDot: "bg-accent-yellow",  label: "bg-accent-yellow/15", labelText: "text-accent-yellow" },
  serving:        { dot: "bg-accent-yellow",  doneDot: "bg-accent-yellow",  label: "bg-accent-yellow/15", labelText: "text-accent-yellow" },
  online:         { dot: "bg-accent-green",   doneDot: "bg-accent-green",   label: "bg-accent-green/15",  labelText: "text-accent-green" },
};

const defaultPhaseColor = { dot: "bg-card-border", doneDot: "bg-accent-green", label: "bg-card-bg", labelText: "text-muted" };

export function StepList({ steps, currentStep, phase }: StepListProps) {
  // Group steps by phase to insert headers
  let lastPhase = "";

  return (
    <div className="space-y-0.5">
      {steps.map((s, i) => {
        const pc = phaseColors[s.phase] ?? defaultPhaseColor;
        const showPhaseHeader = s.phase !== lastPhase;
        lastPhase = s.phase;

        let dotClass = "bg-card-border text-muted";
        if (i < currentStep) dotClass = `${pc.doneDot} text-white`;
        else if (i === currentStep && phase !== "idle" && phase !== "done")
          dotClass = `${pc.dot} text-white animate-pulse-glow`;

        return (
          <div key={i}>
            {showPhaseHeader && (
              <div className={`flex items-center gap-1.5 px-1.5 py-0.5 rounded text-[9px] font-semibold uppercase tracking-wider mt-1 mb-0.5 ${pc.label} ${pc.labelText}`}>
                <span className={`w-1.5 h-1.5 rounded-full ${pc.dot}`} />
                {s.phase}
              </div>
            )}
            <div className="flex items-start gap-2 py-1.5 border-b border-card-border last:border-0">
              <div
                className={`flex-shrink-0 w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-bold mt-0.5 ${dotClass}`}
              >
                {i + 1}
              </div>
              <div className="min-w-0">
                <div className="text-xs leading-snug font-medium">
                  <MathText>{s.name}</MathText>
                </div>
                {s.description && (
                  <div className="text-[10px] text-muted leading-snug mt-0.5">
                    <MathText>{s.description}</MathText>
                  </div>
                )}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
