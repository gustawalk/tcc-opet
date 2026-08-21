import { invoke } from "@tauri-apps/api/core";
import type { LanModeStatus } from "./types";

export interface DataClientOptions {
  mutation?: boolean;
  idempotencyKey?: string;
}

function newIdempotencyKey(): string {
  return globalThis.crypto.randomUUID();
}

export async function dataCommand<T>(
  operation: string,
  payload: Record<string, unknown> = {},
  options: DataClientOptions = {},
): Promise<T> {
  const status = await invoke<LanModeStatus>("get_lan_mode_config");
  if (status.activeMode !== "client") {
    return invoke<T>(operation, payload);
  }

  return invoke<T>("lan_remote_command", {
    operation,
    payload,
    idempotencyKey: options.mutation
      ? (options.idempotencyKey ?? newIdempotencyKey())
      : null,
  });
}

export const dataClient = {
  query<T>(operation: string, payload: Record<string, unknown> = {}) {
    return dataCommand<T>(operation, payload);
  },
  mutate<T>(
    operation: string,
    payload: Record<string, unknown> = {},
    idempotencyKey?: string,
  ) {
    return dataCommand<T>(operation, payload, { mutation: true, idempotencyKey });
  },
};
