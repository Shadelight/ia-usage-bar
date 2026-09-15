export interface UsagePace {
  expectedUsedPercent: number;
  actualUsedPercent: number;
  deltaPercent: number;
  willLastToReset?: boolean | null;
  estimatedExhaustedAt?: string | null;
}

export interface UsageQuota {
  id: string;
  label: string;
  usedPercent?: number | null;
  remainingPercent?: number | null;
  resetAt?: string | null;
  resetInSeconds?: number | null;
  stale?: boolean;
  windowType?: "session" | "5h" | "daily" | "weekly" | "monthly" | "credits" | "custom";
  pace?: UsagePace;
  groupId?: string | null;
  groupLabel?: string | null;
  models?: string[];
  visible?: string;
}

export interface LimitingQuota {
  id: string;
  label: string;
  windowType: string;
  usedPercent: number;
  availablePercent: number;
  resetAt: string | null;
  resetInSeconds: number | null;
  expectedUsedPercent: number | null;
  deltaPercent: number | null;
  estimatedExhaustedAt: string | null;
  exhaustsBeforeResetSeconds: number | null;
}

export interface Recommendation {
  severity: "healthy" | "warning" | "critical";
  action: "stay" | "switch" | "balanced" | "insufficient_data";
  fromId: string;
  toId: string | null;
  toName: string | null;
  reason: string;
  limitingQuota: LimitingQuota | null;
  confidence: number;
}

export interface Provider {
  id: string;
  name: string;
  enabled: boolean;
  stale: boolean;
  updatedAt?: string | null;
  quotas: UsageQuota[];
  credits?: {
    remaining: number;
    resetsAvailable?: number | null;
    fetchedAt?: string | null;
    stale?: boolean;
    resetsStale?: boolean;
  } | null;
}

export interface DashboardSnapshot {
  schemaVersion: number;
  generatedAt: string;
  providers: Provider[];
  recommendation?: Recommendation | null;
}

export interface WatchMessage {
  type: "snapshot";
  source: "initial" | "codex-session" | "remote-poll";
  snapshot: DashboardSnapshot;
}
