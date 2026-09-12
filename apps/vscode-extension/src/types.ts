export interface UsageQuota {
  id: string;
  label: string;
  usedPercent?: number | null;
  resetAt?: string | null;
  resetInSeconds?: number | null;
  stale?: boolean;
}

export interface Provider {
  id: string;
  name: string;
  enabled: boolean;
  stale: boolean;
  updatedAt?: string | null;
  quotas: UsageQuota[];
}

export interface DashboardSnapshot {
  schemaVersion: number;
  generatedAt: string;
  providers: Provider[];
}

export interface WatchMessage {
  type: "snapshot";
  source: "initial" | "codex-session" | "remote-poll";
  snapshot: DashboardSnapshot;
}
