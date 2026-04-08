export interface ActorState {
  session_id: number;
  party_id: number;
  role: string;
  phase: string;
  current_step: number;
  total_steps: number;
  steps: StepDef[];
  status: string;
  log: LogEntry[];
  bytes_sent: number;
  bytes_received_offline: number;
  elapsed_ms: number;
  result: string | null;
  configured: boolean;
  step_requested: boolean;
}

export interface StepDef {
  idx: number;
  name: string;
  phase: string;
  description: string;
}

export interface LogEntry {
  time: number;
  msg: string;
  level: string;
  values?: Record<string, unknown>;
}

export interface ActorConfig {
  name: string;
  url: string;
  color: string;
  icon: string;
}

export interface LambdaShareResponse {
  ready: boolean;
  party_id?: number;
  dim?: number;
  k_bits?: number;
  lambda_a_share?: string[];
  lambda_b_share?: string[];
}
