import { invoke } from "@tauri-apps/api/core";
import type { LanModeStatus } from "./types";

export interface DataClientOptions {
  mutation?: boolean;
  idempotencyKey?: string;
}

const MUTATIONS = new Set([
  "create_customer",
  "update_customer",
  "delete_customer",
  "create_user",
  "update_user",
  "delete_user",
  "create_inventory_item",
  "update_inventory_item",
  "delete_inventory_item",
  "restock_inventory_item",
  "remove_stock_inventory_item",
  "create_checklist_template",
  "update_checklist_template",
  "delete_checklist_template",
  "create_full_service_order",
  "transition_service_order_status",
  "save_service_order_edit",
  "delete_service_order",
  "add_part_to_service_order",
  "remove_part_from_service_order",
  "update_service_order_part_quantity",
  "save_service_order_checklist",
  "upload_service_order_attachment",
  "delete_service_order_attachment",
]);

let activeMode: LanModeStatus["activeMode"] = "local";

export function configureDataClient(mode: LanModeStatus["activeMode"]): void {
  activeMode = mode;
}

export async function initializeDataClient(): Promise<LanModeStatus> {
  const status = await invoke<LanModeStatus>("get_lan_mode_config");
  configureDataClient(status.activeMode);
  return status;
}

function newIdempotencyKey(): string {
  return globalThis.crypto.randomUUID();
}

export async function dataCommand<T>(
  operation: string,
  payload: Record<string, unknown> = {},
  options: DataClientOptions = {},
): Promise<T> {
  if (activeMode !== "client") {
    return invoke<T>(operation, payload);
  }

  const mutation = options.mutation ?? MUTATIONS.has(operation);
  return invoke<T>("lan_remote_command", {
    operation,
    payload,
    idempotencyKey: mutation
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
