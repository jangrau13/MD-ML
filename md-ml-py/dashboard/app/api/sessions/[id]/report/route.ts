import { NextRequest, NextResponse } from "next/server";
import { getSession } from "@/lib/db";

// GET /api/sessions/[id]/report — HTML protocol report for PDF printing
export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const row = getSession(Number(id));

  if (!row) return NextResponse.json({ error: "not found" }, { status: 404 });

  const matA = row.mat_a;
  const matB = row.mat_b;
  const refResult = row.ref_result;
  const deltaA = row.delta_a;
  const deltaB = row.delta_b;
  const mpcResult = row.mpc_result;

  const dim = row.dim;
  const date = new Date(row.created_at * 1000).toISOString();

  function matrixTable(values: string[], label: string, maxShow = 64): string {
    if (dim <= 8) {
      let html = `<table class="matrix"><tbody>`;
      for (let r = 0; r < dim; r++) {
        html += "<tr>";
        for (let c = 0; c < dim; c++) {
          const v = values[r * dim + c];
          html += `<td>${trunc(v)}</td>`;
        }
        html += "</tr>";
      }
      html += "</tbody></table>";
      return html;
    }
    const show = values.slice(0, maxShow);
    let html = `<div class="values">`;
    show.forEach((v, i) => {
      html += `<div><span class="idx">[${i}]</span> ${v}</div>`;
    });
    if (values.length > maxShow) {
      html += `<div class="ellipsis">… ${values.length - maxShow} more elements</div>`;
    }
    html += `</div>`;
    return html;
  }

  function trunc(h: string): string {
    return h.length > 18 ? h.slice(0, 10) + "…" + h.slice(-4) : h;
  }

  function escHtml(s: string): string {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  const reportHtml = `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>MD-ML Protocol Report — Session #${row.id}</title>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css">
<script src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js"></script>
<style>
  @page { margin: 1.5cm; size: A4; }
  body { font-family: 'Segoe UI', system-ui, sans-serif; font-size: 11px; color: #1f2328; max-width: 900px; margin: 0 auto; padding: 20px; }
  h1 { font-size: 18px; color: #0969da; border-bottom: 2px solid #0969da; padding-bottom: 6px; }
  h2 { font-size: 14px; color: #8250df; margin-top: 20px; border-bottom: 1px solid #d0d7de; padding-bottom: 4px; }
  h3 { font-size: 12px; color: #1a7f37; margin-top: 14px; }
  .meta { color: #656d76; font-size: 10px; margin-bottom: 16px; }
  .section { margin: 12px 0; page-break-inside: avoid; }
  table.matrix { border-collapse: collapse; font-family: 'Fira Code', monospace; font-size: 9px; margin: 8px 0; }
  table.matrix td { border: 1px solid #d0d7de; padding: 2px 4px; text-align: right; }
  .values { font-family: 'Fira Code', monospace; font-size: 9px; column-count: 2; margin: 8px 0; }
  .values .idx { color: #656d76; display: inline-block; width: 40px; text-align: right; margin-right: 4px; }
  .ellipsis { color: #656d76; font-style: italic; }
  .muted { color: #656d76; }
  @media print { .no-print { display: none; } }
</style>
</head>
<body>
<h1>MD-ML Protocol Report</h1>
<div class="meta">
  Session #${row.id} — ${dim}×${dim} matrix multiplication — ${date}<br>
  Protocol: \\(\\text{SPD}\\mathbb{Z}_{2^k}\\) — SPDZ-2k
</div>

<button class="no-print" onclick="window.print()" style="padding:8px 16px;background:#0969da;color:white;border:none;border-radius:4px;cursor:pointer;margin-bottom:16px">
  Print / Save as PDF
</button>

<h2>1. Input Matrices</h2>
<div class="section">
  <h3>Matrix \\(A\\)</h3>
  ${matrixTable(matA, "A")}

  <h3>Matrix \\(B\\)</h3>
  ${matrixTable(matB, "B")}

  <h3>Reference Result \\(C = A \\times B\\)</h3>
  ${matrixTable(refResult, "C")}
</div>

<h2>2. Masked Inputs</h2>
<div class="section">
  ${deltaA ? `<h3>\\(\\Delta_A\\) (sent to parties)</h3>${matrixTable(deltaA, "Δ_A")}` : ""}
  ${deltaB ? `<h3>\\(\\Delta_B\\) (sent to parties)</h3>${matrixTable(deltaB, "Δ_B")}` : ""}
</div>

<h2>3. MPC Result</h2>
<div class="section">
  ${mpcResult
    ? `${matrixTable(mpcResult, "MPC Result")}`
    : '<p class="muted">MPC result not yet available.</p>'
  }
</div>

<script>
  document.addEventListener("DOMContentLoaded", function() {
    renderMathInElement(document.body, {
      delimiters: [
        {left: "\\\\(", right: "\\\\)", display: false},
        {left: "\\\\[", right: "\\\\]", display: true},
      ]
    });
  });
</script>
</body>
</html>`;

  return new NextResponse(reportHtml, {
    headers: { "Content-Type": "text/html" },
  });
}
