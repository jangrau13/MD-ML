"use client";

import { useState, useCallback, useEffect, useRef } from "react";
import { Session } from "../components/ReferencePanel";
import {
  PARTY0_URL, PARTY1_URL, DP_URLS,
  NUM_DATA_PARTIES, columnsForDataParty, makeMask,
} from "../lib/constants";

export interface CoordinatorState {
  configured: boolean;
  configStatus: string;
  sessions: Session[];
  activeSessionId: number | null;
  activeSession: Session | null;
  allDpsDone: boolean;
  configRef: React.MutableRefObject<{ dim: number; d: number; k: number }>;
  sessionRef: React.MutableRefObject<{ allDpDone: boolean; [key: string]: unknown }>;
}

export interface CoordinatorActions {
  handleConfigure: (dim: number, d: number, k: number) => Promise<void>;
  handleReset: () => void;
  setActiveSessionId: (id: number) => void;
  stepBothParties: () => Promise<void>;
  stepAllDataParties: () => Promise<void>;
  runComputationParties: () => Promise<void>;
  runDataParties: () => Promise<void>;
}

export function useCoordinator(
  dpActors: { state: { phase?: string; current_step?: number } | null }[],
) {
  const [configured, setConfigured] = useState(false);
  const [configStatus, setConfigStatus] = useState("");
  const [sessions, setSessions] = useState<Session[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<number | null>(null);

  const configRef = useRef<{ dim: number; d: number; k: number }>({ dim: 4, d: 20, k: 64 });
  const sessionRef = useRef<{
    sessionId?: number;
    floatA?: number[];
    floatB?: number[];
    floatRef?: number[];
    matA?: string[];
    matB?: string[];
    refResult?: string[];
    deltaA?: string[];
    deltaB?: string[];
    allDpDone: boolean;
  }>({ allDpDone: false });
  const assembledRef = useRef(false);

  // Load existing sessions
  useEffect(() => {
    fetch("/api/sessions")
      .then((r) => {
        if (!r.ok) throw new Error("API error");
        return r.json();
      })
      .then((rows: { id: number; dim: number; created_at: number }[]) => {
        if (rows && rows.length > 0) {
          Promise.all(
            rows.map((r) =>
              fetch(`/api/sessions/${r.id}`).then((res) => res.json())
            )
          ).then((fullSessions) => {
            const mapped: Session[] = fullSessions.map((s) => ({
              id: s.id,
              dim: s.dim,
              d: s.d ?? 20,
              floatA: s.float_a ?? [],
              floatB: s.float_b ?? [],
              floatRef: s.float_ref ?? [],
              matA: s.mat_a,
              matB: s.mat_b,
              refResult: s.ref_result,
              timestamp: s.created_at * 1000,
            }));
            setSessions(mapped);
            if (mapped.length > 0) setActiveSessionId(mapped[0].id);
          });
        }
      })
      .catch(() => {});
  }, []);

  const activeSession = sessions.find((s) => s.id === activeSessionId) ?? null;

  const { dim } = configRef.current;
  const activeDpCount = Array.from({ length: NUM_DATA_PARTIES }, (_, i) => i)
    .filter((i) => columnsForDataParty(i, dim).length > 0).length;
  const allDpsDone = dpActors
    .slice(0, activeDpCount)
    .every((dp) => dp.state?.phase === "done");

  // Assemble matrices from data party /api/delta and send to computation parties
  const assembleAndSend = useCallback(async () => {
    const { dim, d, k } = configRef.current;
    const size = dim * dim;

    const floatA = new Array<number>(size).fill(0);
    const floatB = new Array<number>(size).fill(0);
    const matA = new Array<string>(size).fill("0");
    const matB = new Array<string>(size).fill("0");
    const deltaA = new Array<string>(size).fill("0");
    const deltaB = new Array<string>(size).fill("0");

    for (let dpId = 0; dpId < NUM_DATA_PARTIES; dpId++) {
      const cols = columnsForDataParty(dpId, dim);
      if (cols.length === 0) continue;
      try {
        const resp = await fetch(`${DP_URLS[dpId]}/api/delta`);
        const data = await resp.json();
        if (!data.ready) continue;
        let idx = 0;
        for (let row = 0; row < dim; row++) {
          for (const col of cols) {
            const flatIdx = row * dim + col;
            floatA[flatIdx] = data.float_col_a[idx];
            floatB[flatIdx] = data.float_col_b[idx];
            matA[flatIdx] = data.fixed_col_a[idx];
            matB[flatIdx] = data.fixed_col_b[idx];
            deltaA[flatIdx] = data.delta_col_a[idx];
            deltaB[flatIdx] = data.delta_col_b[idx];
            idx++;
          }
        }
      } catch { /* skip */ }
    }

    const MASK = makeMask(k);
    const half = BigInt(1) << BigInt(k - 1);
    const mod = BigInt(1) << BigInt(k);
    const floatRef: number[] = [];
    const refResult: string[] = [];
    for (let i = 0; i < dim; i++) {
      for (let j = 0; j < dim; j++) {
        let isum = BigInt(0);
        for (let m = 0; m < dim; m++) {
          isum = (isum + BigInt(matA[i * dim + m]) * BigInt(matB[m * dim + j])) & MASK;
        }
        const truncated = (isum >> BigInt(d)) & MASK;
        refResult.push(truncated.toString());
        // Convert back to float: interpret as signed, divide by 2^d
        let signed = truncated;
        if (signed >= half) signed -= mod;
        floatRef.push(Number(signed) / (2 ** d));
      }
    }

    const resp = await fetch("/api/sessions", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        dim, d,
        mat_a: matA, mat_b: matB, ref_result: refResult,
        float_a: floatA, float_b: floatB, float_ref: floatRef,
      }),
    });
    const { id: sessionId } = await resp.json();

    const newSession: Session = {
      id: sessionId as number, dim, d,
      floatA, floatB, floatRef,
      matA, matB, refResult,
      timestamp: Date.now(),
    };
    setSessions((prev) => [newSession, ...prev]);
    setActiveSessionId(sessionId as number);
    sessionRef.current = { sessionId, floatA, floatB, floatRef, matA, matB, refResult, deltaA, deltaB, allDpDone: true };

    await fetch(`/api/sessions/${sessionId}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ delta_a: deltaA, delta_b: deltaB, status: "running" }),
    });

    // Send deltas to computation parties via /api/deltas (not /api/configure)
    const deltaPayload = { delta_a: deltaA, delta_b: deltaB };
    await Promise.all([
      fetch(`${PARTY0_URL}/api/deltas`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(deltaPayload),
      }),
      fetch(`${PARTY1_URL}/api/deltas`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(deltaPayload),
      }),
    ]);

    setConfigStatus(`Ready! ${dim}\u00D7${dim}, d=${d}. Step through computation parties.`);
  }, []);

  // Poll DPs for float data and create a preview session as soon as available
  const previewRef = useRef(false);
  useEffect(() => {
    if (!configured || previewRef.current) return;
    const { dim, d } = configRef.current;
    const size = dim * dim;
    const interval = setInterval(async () => {
      if (previewRef.current) { clearInterval(interval); return; }
      const floatA = new Array<number>(size).fill(0);
      const floatB = new Array<number>(size).fill(0);
      let gotAny = false;
      for (let dpId = 0; dpId < NUM_DATA_PARTIES; dpId++) {
        const cols = columnsForDataParty(dpId, dim);
        if (cols.length === 0) continue;
        try {
          const resp = await fetch(`${DP_URLS[dpId]}/api/delta`);
          const data = await resp.json();
          if (!data.float_col_a) continue;
          gotAny = true;
          let idx = 0;
          for (let row = 0; row < dim; row++) {
            for (const col of cols) {
              floatA[row * dim + col] = data.float_col_a[idx];
              floatB[row * dim + col] = data.float_col_b[idx];
              idx++;
            }
          }
        } catch { /* skip */ }
      }
      if (!gotAny) return;
      // Check if ALL active DPs have contributed floats
      const allHaveFloats = Array.from({ length: NUM_DATA_PARTIES }, (_, i) => i)
        .filter((i) => columnsForDataParty(i, dim).length > 0)
        .every(async (dpId) => {
          try {
            const resp = await fetch(`${DP_URLS[dpId]}/api/delta`);
            const data = await resp.json();
            return !!data.float_col_a;
          } catch { return false; }
        });
      if (!allHaveFloats) return;

      previewRef.current = true;
      clearInterval(interval);

      // Encode floats to fixed-point, compute C in fixed-point, convert back
      const { k } = configRef.current;
      const fmod = BigInt(1) << BigInt(k);
      const fmask = fmod - BigInt(1);
      const fhalf = fmod >> BigInt(1);
      const scale = 2 ** d;

      function toFixed(x: number): bigint {
        const scaled = Math.round(x * scale);
        return BigInt(scaled < 0 ? Number(fmod) + scaled : scaled) & fmask;
      }

      // Encode A and B
      const fixA = floatA.map(toFixed);
      const fixB = floatB.map(toFixed);

      // Compute C = A × B in fixed-point mod 2^k, then truncate by 2^d
      const floatRef: number[] = [];
      for (let i = 0; i < dim; i++) {
        for (let j = 0; j < dim; j++) {
          let isum = BigInt(0);
          for (let m = 0; m < dim; m++) {
            isum = (isum + fixA[i * dim + m] * fixB[m * dim + j]) & fmask;
          }
          const truncated = (isum >> BigInt(d)) & fmask;
          let signed = truncated;
          if (signed >= fhalf) signed -= fmod;
          floatRef.push(Number(signed) / scale);
        }
      }

      // Create preview session
      const preview: Session = {
        id: -1, dim, d,
        floatA, floatB, floatRef,
        matA: [], matB: [], refResult: [],
        timestamp: Date.now(),
      };
      setSessions((prev) => {
        const without = prev.filter((s) => s.id !== -1);
        return [preview, ...without];
      });
      setActiveSessionId(-1);
    }, 800);
    return () => clearInterval(interval);
  }, [configured]);

  // Auto-assemble when all DPs done
  useEffect(() => {
    if (allDpsDone && configured && !assembledRef.current) {
      assembledRef.current = true;
      assembleAndSend();
    }
  }, [allDpsDone, configured, assembleAndSend]);

  const handleConfigure = useCallback(
    async (selectedDim: number, selectedD: number, selectedK: number) => {
      configRef.current = { dim: selectedDim, d: selectedD, k: selectedK };
      assembledRef.current = false;
      sessionRef.current = { allDpDone: false };

      // Configure ALL parties (data + computation) with dim, d, k
      // Computation parties start preprocessing immediately; deltas arrive later
      const partyConfig = { dim: selectedDim, d: selectedD, k: selectedK };
      await Promise.all([
        ...DP_URLS.map((url) =>
          fetch(`${url}/api/configure`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(partyConfig),
          }).catch(() => {})
        ),
        fetch(`${PARTY0_URL}/api/configure`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(partyConfig),
        }).catch(() => {}),
        fetch(`${PARTY1_URL}/api/configure`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(partyConfig),
        }).catch(() => {}),
      ]);

      setConfigured(true);
      setConfigStatus(`Configured ${selectedDim}\u00D7${selectedDim}, d=${selectedD}, k=${selectedK}. Step through each Data Party.`);
    },
    []
  );

  const handleReset = useCallback(async () => {
    setConfigured(false);
    setConfigStatus("");
    assembledRef.current = false;
    sessionRef.current = { allDpDone: false };

    // Reset all backend parties and data parties
    await Promise.all([
      ...DP_URLS.map((url) =>
        fetch(`${url}/api/reset`, { method: "POST" }).catch(() => {})
      ),
      fetch(`${PARTY0_URL}/api/reset`, { method: "POST" }).catch(() => {}),
      fetch(`${PARTY1_URL}/api/reset`, { method: "POST" }).catch(() => {}),
    ]);
  }, []);

  // Auto-run: step both computation parties until done (or blocked)
  const [compRunning, setCompRunning] = useState(false);
  const runComputationParties = useCallback(async () => {
    setCompRunning(true);
    // Keep stepping both parties until they're done or we hit a block
    for (let i = 0; i < 100; i++) { // safety limit
      await Promise.all([
        fetch(`${PARTY0_URL}/api/step`, { method: "POST" }).catch(() => {}),
        fetch(`${PARTY1_URL}/api/step`, { method: "POST" }).catch(() => {}),
      ]);
      // Wait for state to update
      await new Promise((r) => setTimeout(r, 600));
      // Check if both are done or waiting for something
      try {
        const [s0, s1] = await Promise.all([
          fetch(`${PARTY0_URL}/api/state`).then((r) => r.json()),
          fetch(`${PARTY1_URL}/api/state`).then((r) => r.json()),
        ]);
        if (s0.phase === "done" && s1.phase === "done") break;
        if (!s0.step_requested && !s1.step_requested) {
          // Both are waiting for a step — they consumed the last one, continue
        }
      } catch { break; }
    }
    setCompRunning(false);
  }, []);

  // Auto-run: step all active data parties until done
  const [dpRunning, setDpRunning] = useState(false);
  const runDataParties = useCallback(async () => {
    setDpRunning(true);
    const { dim } = configRef.current;
    const activeUrls = Array.from({ length: NUM_DATA_PARTIES }, (_, i) => i)
      .filter((i) => columnsForDataParty(i, dim).length > 0)
      .map((i) => DP_URLS[i]);

    for (let i = 0; i < 100; i++) {
      await Promise.all(
        activeUrls.map((url) =>
          fetch(`${url}/api/step`, { method: "POST" }).catch(() => {})
        )
      );
      await new Promise((r) => setTimeout(r, 600));
      try {
        const states = await Promise.all(
          activeUrls.map((url) => fetch(`${url}/api/state`).then((r) => r.json()))
        );
        if (states.every((s) => s.phase === "done")) break;
      } catch { break; }
    }
    setDpRunning(false);
  }, []);

  // Single-step: advance both computation parties by one step
  const stepBothParties = useCallback(async () => {
    await Promise.all([
      fetch(`${PARTY0_URL}/api/step`, { method: "POST" }).catch(() => {}),
      fetch(`${PARTY1_URL}/api/step`, { method: "POST" }).catch(() => {}),
    ]);
  }, []);

  // Single-step: advance all active data parties by one step
  const stepAllDataParties = useCallback(async () => {
    const { dim } = configRef.current;
    const activeUrls = Array.from({ length: NUM_DATA_PARTIES }, (_, i) => i)
      .filter((i) => columnsForDataParty(i, dim).length > 0)
      .map((i) => DP_URLS[i]);
    await Promise.all(
      activeUrls.map((url) =>
        fetch(`${url}/api/step`, { method: "POST" }).catch(() => {})
      )
    );
  }, []);

  return {
    state: {
      configured,
      configStatus,
      sessions,
      activeSessionId,
      activeSession,
      allDpsDone,
      configRef,
      sessionRef,
    },
    actions: {
      handleConfigure,
      handleReset,
      setActiveSessionId,
      stepBothParties,
      stepAllDataParties,
      runComputationParties,
      runDataParties,
    },
    running: {
      compRunning,
      dpRunning,
    },
  };
}
