"use client";

import { Math } from "./KaTeX";

/**
 * Renders a string with inline KaTeX math.
 *
 * - Anything explicitly between $...$ is rendered as KaTeX.
 * - Otherwise, auto-detects protocol notation and wraps in math mode.
 */
export function MathText({
  children,
  className = "",
}: {
  children: string;
  className?: string;
}) {
  const withMath = autoMathify(children);
  const parts = withMath.split(/(\$[^$]+\$)/g);

  return (
    <span className={className}>
      {parts.map((part, i) => {
        if (part.startsWith("$") && part.endsWith("$")) {
          const tex = part.slice(1, -1);
          return <Math key={i} tex={tex} />;
        }
        return <span key={i}>{part}</span>;
      })}
    </span>
  );
}

function autoMathify(text: string): string {
  // Already has explicit $ delimiters — pass through
  if (text.includes("$")) return text;

  let r = text;

  // ── Full equation patterns (catch these first before breaking into pieces) ──

  // "mod 2^{k+s}" or "mod 2^k" or "mod 2^64" or "mod 2^{64}"
  r = r.replace(/\bmod\s*2\^(\{[^}]+\}|\d+|\w+)/g, (_, e) => {
    const clean = e.replace(/^\{|\}$/g, "");
    return `$\\bmod 2^{${clean}}$`;
  });

  // "2^d" "2^k" "2^{k+s}" "2^{2d}" "2^20" standalone
  r = r.replace(/(?<!\w)2\^(\{[^}]+\}|\d+|[a-z])/g, (_, e) => {
    const clean = e.replace(/^\{|\}$/g, "");
    return `$2^{${clean}}$`;
  });

  // ── Functionality/procedure names ──
  r = r.replace(/F_Prep\.(\w+)/g, (_, s) => `$\\mathcal{F}_{\\text{Prep}}.\\text{${s}}$`);
  r = r.replace(/F_edaBits/g, "$\\mathcal{F}_{\\text{edaBits}}$");
  r = r.replace(/π_MultTrunc/g, "$\\pi_{\\text{MultTrunc}}$");
  r = r.replace(/π_(\w+)/g, (_, s) => `$\\pi_{\\text{${s}}}$`);
  r = r.replace(/Π_(\w+)/g, (_, s) => `$\\Pi_{\\text{${s}}}$`);

  // ── Party names ──
  r = r.replace(/\bP_(\d)/g, (_, s) => `$P_{${s}}$`);

  // ── Greek letters with subscripts/superscripts ──
  // α^0, α^1, α^i, α^{...}
  r = r.replace(/α\^(\{[^}]+\}|\w+)/g, (_, s) => {
    const clean = s.replace(/^\{|\}$/g, "");
    return `$\\alpha^{${clean}}$`;
  });
  r = r.replace(/(?<![\\$\w])α(?![\^_\w])/g, "$\\alpha$");

  // λ_{z'}, λ_z, λ_A, λ_B etc.
  r = r.replace(/λ_\{([^}]+)\}/g, (_, s) => `$\\lambda_{${s.replace(/'/g, "'")}}$`);
  r = r.replace(/λ_([A-Za-z0-9']+)/g, (_, s) => `$\\lambda_{${s}}$`);
  r = r.replace(/(?<![\\$\w])λ(?![\^_{A-Za-z0-9])/g, "$\\lambda$");

  // δ_x, δ_y
  r = r.replace(/δ_([A-Za-z0-9]+)/g, (_, s) => `$\\delta_{${s}}$`);

  // Δ_{z'}, Δ_z, Δ_A, Δ_B
  r = r.replace(/Δ_\{([^}]+)\}/g, (_, s) => `$\\Delta_{${s.replace(/'/g, "'")}}$`);
  r = r.replace(/Δ_([A-Za-z0-9']+)/g, (_, s) => `$\\Delta_{${s}}$`);
  r = r.replace(/(?<![\\$\w])Δ(?![\^_{A-Za-z0-9])/g, "$\\Delta$");

  // Σ
  r = r.replace(/Σ/g, "$\\sum$");

  // ── Share notation: [...] brackets ──
  // [λ_{z'}], [λ_z], [λ_A] etc.
  r = r.replace(/\[λ_\{([^}]+)\}\]/g, (_, s) => `$[\\lambda_{${s}}]$`);
  r = r.replace(/\[λ_([A-Za-z0-9']+)\]/g, (_, s) => `$[\\lambda_{${s}}]$`);

  // [Δ_{z'}], [Δ_z] etc.
  r = r.replace(/\[Δ_\{([^}]+)\}\]/g, (_, s) => `$[\\Delta_{${s}}]$`);
  r = r.replace(/\[Δ_([A-Za-z0-9']+)\]/g, (_, s) => `$[\\Delta_{${s}}]$`);

  // [a], [b], [c], [u]
  r = r.replace(/\[([a-cu])\]/g, (_, s) => `$[${s}]$`);

  // [a_mac], [b_mac], [c_mac]
  r = r.replace(/\[([a-c])_mac\]/g, (_, s) => `$[${s}_{\\text{mac}}]$`);

  // [Δ_{z'}_mac] etc.
  r = r.replace(/\[Δ_\{([^}]+)\}_mac\]/g, (_, s) => `$[\\Delta_{${s},\\text{mac}}]$`);

  // ── Rings ──
  // Z_{2^{k+s}}, Z_{2^k}, Z_{2^64}, Z_{2^{64}}
  r = r.replace(/Z_\{2\^(\{[^}]+\}|\w+)\}/g, (_, exp) => {
    const clean = exp.replace(/^\{|\}$/g, "");
    return `$\\mathbb{Z}_{2^{${clean}}}$`;
  });

  // ── Misc notation ──
  r = r.replace(/∈/g, "$\\in$");
  r = r.replace(/≈/g, "$\\approx$");
  r = r.replace(/✓/g, " $\\checkmark$");

  // ⌊...⌋ floor notation
  r = r.replace(/⌊([^⌋]+)⌋/g, (_, inner) => `$\\lfloor ${inner} \\rfloor$`);

  // n×n
  r = r.replace(/(\d+)×(\d+)/g, (_, a, b) => `$${a} \\times ${b}$`);
  r = r.replace(/n×n/g, "$n \\times n$");

  // temp_xy, temp_x, temp_y → t_{xy}, t_x, t_y
  r = r.replace(/\btemp_xy\b/g, "$t_{xy}$");
  r = r.replace(/\btemp_x\b/g, "$t_x$");
  r = r.replace(/\btemp_y\b/g, "$t_y$");

  // x^i, m^i
  r = r.replace(/(?<!\w)x\^i/g, "$x^i$");
  r = r.replace(/(?<!\w)m\^i/g, "$m^i$");

  // a·b style
  r = r.replace(/(\w)·(\w)/g, (_, a, b) => `$${a} \\cdot ${b}$`);

  // ── Merge adjacent math spans ──
  // "$...$  $...$" → "$... \\; ...$"
  r = r.replace(/\$\s*\$/g, " \\; ");

  return r;
}
