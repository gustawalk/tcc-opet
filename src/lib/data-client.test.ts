import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { dataClient } from "./data-client";

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
  });

  it.each(["local", "host"] as const)(
    "preserves local invoke arguments in %s mode",
    async (activeMode) => {
      mockedInvoke
        .mockResolvedValueOnce(mode(activeMode))
        .mockResolvedValueOnce({ total: 1 });

      await expect(
        dataClient.query("get_customers_page", { limit: 20, offset: 0 }),
      ).resolves.toEqual({ total: 1 });
      expect(mockedInvoke).toHaveBeenNthCalledWith(2, "get_customers_page", {
        limit: 20,
        offset: 0,
      });
    },
  );

  it("routes client reads through the Rust transport", async () => {
    mockedInvoke
      .mockResolvedValueOnce(mode("client"))
      .mockResolvedValueOnce({ items: [], total: 0 });

    await dataClient.query("get_customers_page", { limit: 20 });

    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "lan_remote_command", {
      operation: "get_customers_page",
      payload: { limit: 20 },
      idempotencyKey: null,
    });
  });

  it("adds a stable idempotency key to remote mutations", async () => {
    mockedInvoke
      .mockResolvedValueOnce(mode("client"))
      .mockResolvedValueOnce("customer-id");

    await dataClient.mutate("create_customer", { name: "Ana" }, "request-123");

    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "lan_remote_command", {
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
    mockedInvoke.mockResolvedValueOnce(mode("client")).mockRejectedValueOnce(error);

    await expect(dataClient.query("get_dashboard_data")).rejects.toEqual(error);
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
  });
});
