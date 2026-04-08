interface SessionData {
  id: number;
  dim: number;
  d: number;
  created_at: number;
  status: string;
  float_a: number[];
  float_b: number[];
  float_ref: number[];
  mat_a: string[];
  mat_b: string[];
  ref_result: string[];
  delta_a: string[] | null;
  delta_b: string[] | null;
  mpc_result: string[] | null;
}

let nextId = 1;
const sessions: SessionData[] = [];

export function listSessions(): { id: number; dim: number; created_at: number; status: string }[] {
  return sessions.map(({ id, dim, created_at, status }) => ({
    id, dim, created_at, status,
  })).reverse();
}

export function getSession(id: number): SessionData | null {
  return sessions.find((s) => s.id === id) ?? null;
}

export function createSession(data: { dim: number; d: number; mat_a: string[]; mat_b: string[]; ref_result: string[]; float_a: number[]; float_b: number[]; float_ref: number[] }): number {
  const id = nextId++;
  sessions.push({
    id,
    dim: data.dim,
    d: data.d,
    created_at: Date.now() / 1000,
    status: "configuring",
    float_a: data.float_a,
    float_b: data.float_b,
    float_ref: data.float_ref,
    mat_a: data.mat_a,
    mat_b: data.mat_b,
    ref_result: data.ref_result,
    delta_a: null,
    delta_b: null,
    mpc_result: null,
  });
  return id;
}

export function updateSession(id: number, updates: Partial<SessionData>) {
  const idx = sessions.findIndex((s) => s.id === id);
  if (idx >= 0) {
    sessions[idx] = { ...sessions[idx], ...updates };
  }
}

export type { SessionData };
