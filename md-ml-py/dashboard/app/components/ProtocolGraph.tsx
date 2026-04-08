"use client";

import { useEffect, useRef } from "react";
import * as d3 from "d3";
import { ActorState } from "./types";
import { DATA_PARTY_CONFIGS, columnsForDataParty } from "../lib/constants";

interface ProtocolGraphProps {
  party0: ActorState | null;
  party1: ActorState | null;
  dpActors: { state: ActorState | null }[];
  dim: number;
}

interface NodeData {
  id: string;
  label: string;
  x: number;
  y: number;
  color: string;
  phase: string;
  progress: number;
  radius: number;
}

interface LinkData {
  source: string;
  target: string;
  label: string;
  active: boolean;
  labelOffsetX: number;
  labelOffsetY: number;
}

export function ProtocolGraph({ party0, party1, dpActors, dim }: ProtocolGraphProps) {
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (!svgRef.current) return;

    const width = svgRef.current.clientWidth || 420;
    const height = 180;
    const cx = width / 2;

    //  Layout:
    //   Party 0  ←──OT / shares──→  Party 1     (top row, computation)
    //    ↕  ↕                         ↕  ↕
    //   DP0  DP1                    DP2  DP3      (bottom row, data)

    // Computation parties (top)
    const nodes: NodeData[] = [
      {
        id: "party0", label: "Party 0",
        x: cx - 100, y: 35,
        color: "#0969da",
        phase: party0?.phase ?? "idle",
        progress: party0 ? party0.current_step / Math.max(party0.total_steps, 1) : 0,
        radius: 20,
      },
      {
        id: "party1", label: "Party 1",
        x: cx + 100, y: 35,
        color: "#1a7f37",
        phase: party1?.phase ?? "idle",
        progress: party1 ? party1.current_step / Math.max(party1.total_steps, 1) : 0,
        radius: 20,
      },
    ];

    // Data parties (bottom row, evenly spaced)
    const activeDps = DATA_PARTY_CONFIGS.filter((_, i) => columnsForDataParty(i, dim).length > 0);
    const dpSpacing = width / (activeDps.length + 1);
    for (let i = 0; i < activeDps.length; i++) {
      const dp = activeDps[i];
      const dpState = dpActors[dp.id]?.state;
      nodes.push({
        id: `dp${dp.id}`,
        label: dp.name,
        x: dpSpacing * (i + 1),
        y: 140,
        color: dp.color,
        phase: dpState?.phase ?? "idle",
        progress: dpState ? dpState.current_step / Math.max(dpState.total_steps, 1) : 0,
        radius: 15,
      });
    }

    const partiesOnline = party0?.phase === "online" || party1?.phase === "online";
    const partiesPreprocessing = party0?.phase === "preprocessing" || party1?.phase === "preprocessing";

    const links: LinkData[] = [
      // Party 0 ↔ Party 1 (OT for preprocessing, shares for online)
      {
        source: "party0", target: "party1",
        label: partiesOnline ? "shares" : partiesPreprocessing ? "OT (SPDZ-2k)" : "OT / shares",
        active: partiesOnline || partiesPreprocessing,
        labelOffsetX: 0, labelOffsetY: -8,
      },
    ];

    // Data party → computation party links (λ shares up, Δ down)
    for (const dp of activeDps) {
      const dpState = dpActors[dp.id]?.state;
      const dpDone = dpState?.phase === "done";
      const dpOnline = dpState?.phase === "online";
      // Each DP connects to both computation parties
      links.push({
        source: `dp${dp.id}`, target: "party0",
        label: dpDone ? "Δ" : dpOnline ? "[λ]^0" : "",
        active: dpOnline || dpDone,
        labelOffsetX: -8, labelOffsetY: 0,
      });
      links.push({
        source: `dp${dp.id}`, target: "party1",
        label: dpDone ? "Δ" : dpOnline ? "[λ]^1" : "",
        active: dpOnline || dpDone,
        labelOffsetX: 8, labelOffsetY: 0,
      });
    }

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();
    svg.attr("viewBox", `0 0 ${width} ${height}`);

    // Links
    svg.selectAll("line")
      .data(links)
      .enter()
      .append("line")
      .attr("x1", (d) => nodes.find((n) => n.id === d.source)!.x)
      .attr("y1", (d) => nodes.find((n) => n.id === d.source)!.y)
      .attr("x2", (d) => nodes.find((n) => n.id === d.target)!.x)
      .attr("y2", (d) => nodes.find((n) => n.id === d.target)!.y)
      .attr("stroke", (d) => (d.active ? "#0969da" : "#d0d7de"))
      .attr("stroke-width", (d) => (d.active ? 1.5 : 1))
      .attr("stroke-dasharray", (d) => (d.active ? "none" : "3,3"));

    // Link labels (only for the OT/shares label between computation parties)
    svg.selectAll(".link-label")
      .data(links.filter((l) => l.label))
      .enter()
      .append("text")
      .attr("x", (d) => {
        const s = nodes.find((n) => n.id === d.source)!;
        const t = nodes.find((n) => n.id === d.target)!;
        return (s.x + t.x) / 2 + d.labelOffsetX;
      })
      .attr("y", (d) => {
        const s = nodes.find((n) => n.id === d.source)!;
        const t = nodes.find((n) => n.id === d.target)!;
        return (s.y + t.y) / 2 + d.labelOffsetY;
      })
      .attr("text-anchor", "middle")
      .attr("fill", (d) => (d.active ? "#656d76" : "#d0d7de"))
      .attr("font-size", "7px")
      .attr("font-family", "var(--font-geist-mono)")
      .text((d) => d.label);

    // Nodes
    const nodeG = svg.selectAll(".node")
      .data(nodes)
      .enter()
      .append("g")
      .attr("transform", (d) => `translate(${d.x},${d.y})`);

    nodeG.append("circle")
      .attr("r", (d) => d.radius)
      .attr("fill", "#ffffff")
      .attr("stroke", (d) => d.color)
      .attr("stroke-width", 2);

    const arc = d3.arc<NodeData>()
      .innerRadius((d) => d.radius - 2)
      .outerRadius((d) => d.radius + 1)
      .startAngle(0)
      .endAngle((d) => d.progress * 2 * Math.PI);

    nodeG.append("path")
      .attr("d", arc as never)
      .attr("fill", (d) => d.color)
      .attr("opacity", 0.8);

    // Phase text inside node
    nodeG.append("text")
      .attr("y", 3)
      .attr("text-anchor", "middle")
      .attr("fill", "#1f2328")
      .attr("font-size", (d) => d.radius > 16 ? "7px" : "6px")
      .attr("font-family", "var(--font-geist-mono)")
      .text((d) => {
        if (d.phase === "idle") return "wait";
        if (d.phase === "done") return "done";
        return d.phase.slice(0, 5);
      });

    // Labels below
    nodeG.append("text")
      .attr("y", (d) => d.radius + 12)
      .attr("text-anchor", "middle")
      .attr("fill", (d) => d.color)
      .attr("font-size", (d) => d.radius > 16 ? "9px" : "8px")
      .attr("font-weight", "600")
      .text((d) => d.label);
  }, [party0, party1, dpActors, dim]);

  return <svg ref={svgRef} className="w-full" style={{ height: 180 }} />;
}
