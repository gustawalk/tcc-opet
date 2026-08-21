import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { configureDataClient, dataClient, initializeDataClient } from "./data-client";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

function mode(activeMode: "local" | "host" | "client") {
  return {
    config: { mode: activeMode, hostPort: 8743 },
    activeMode,
    restartRequired: false,
    storageReady: activeMode !== "client",
  };
}

describe("dataClient", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    configureDataClient("local");
  });

  afterEach(() => configureDataClient("local"));

  it.each(["local", "host"] as const)(
    "preserves local invoke arguments in %s mode",
    async (activeMode) => {
      configureDataClient(activeMode);
      mockedInvoke.mockResolvedValueOnce({ total: 1 });

      await expect(
        dataClient.query("get_customers_page", { limit: 20, offset: 0 }),
      ).resolves.toEqual({ total: 1 });
      expect(mockedInvoke).toHaveBeenCalledWith("get_customers_page", {
        limit: 20,
        offset: 0,
      });
    },
  );

  it("routes client reads through the Rust transport", async () => {
    configureDataClient("client");
    mockedInvoke.mockResolvedValueOnce({ items: [], total: 0 });

    await dataClient.query("get_customers_page", { limit: 20 });

    expect(mockedInvoke).toHaveBeenCalledWith("lan_remote_command", {
      operation: "get_customers_page",
      payload: { limit: 20 },
      idempotencyKey: null,
    });
  });

  it("adds a stable idempotency key to remote mutations", async () => {
    configureDataClient("client");
    mockedInvoke.mockResolvedValueOnce("customer-id");

    await dataClient.mutate("create_customer", { name: "Ana" }, "request-123");

    expect(mockedInvoke).toHaveBeenCalledWith("lan_remote_command", {
      operation: "create_customer",
      payload: { name: "Ana" },
      idempotencyKey: "request-123",
    });
  });

  it.each([
    { en: "Unauthorized", pt: "Dispositivo revogado." },
    { en: "Unreachable", pt: "O computador host está indisponível." },
    { en: "Certificate changed", pt: "O certificado do host mudou." },
    { en: "Version mismatch", pt: "As versões devem ser iguais." },
    { en: "Invalid input", pt: "Os dados enviados são inválidos." },
  ])("preserves typed remote errors: $en", async (error) => {
    configureDataClient("client");
    mockedInvoke.mockRejectedValueOnce(error);

    await expect(dataClient.query("get_dashboard_data")).rejects.toEqual(error);
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
  });

  it("initializes the active mode before application routes render", async () => {
    mockedInvoke.mockResolvedValueOnce(mode("client"));

    await expect(initializeDataClient()).resolves.toEqual(mode("client"));
    mockedInvoke.mockResolvedValueOnce({ ok: true });
    await dataClient.query("get_dashboard_data");

    expect(mockedInvoke).toHaveBeenLastCalledWith("lan_remote_command", {
      operation: "get_dashboard_data",
      payload: {},
      idempotencyKey: null,
    });
  });
});
