import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AutomaticBackupProgress } from "@/components/shared/AutomaticBackupProgress";
import type { AutomaticBackupProgress as ProgressEvent } from "@/lib/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);
let eventHandler: ((event: { payload: ProgressEvent }) => void) | undefined;

afterEach(() => {
  cleanup();
  eventHandler = undefined;
  vi.clearAllMocks();
});

describe("AutomaticBackupProgress", () => {
  it("recovers an already running backup from backend status", async () => {
    mockedInvoke.mockResolvedValue({
      running: true,
      progressPercent: 30,
      phase: "exporting",
    });
    mockedListen.mockResolvedValue(vi.fn());

    render(<AutomaticBackupProgress />);

    expect(await screen.findByRole("status", { name: "Backup automático em andamento" })).toBeInTheDocument();
    expect(screen.getByText(/30% concluído/)).toBeInTheDocument();
  });

  it("shows progress events globally and closes only after completion", async () => {
    mockedInvoke.mockResolvedValue({ running: false });
    mockedListen.mockImplementation((_event, handler) => {
      eventHandler = handler as unknown as (event: { payload: ProgressEvent }) => void;
      return Promise.resolve(vi.fn());
    });
    render(<AutomaticBackupProgress />);
    await waitFor(() => expect(eventHandler).toBeDefined());

    act(() => {
      eventHandler?.({
        payload: {
          running: true,
          percent: 80,
          phase: "validating",
          message: "Validando a integridade do backup.",
        },
      });
    });
    expect(screen.getByRole("status")).toHaveTextContent("80% concluído");
    expect(screen.getByRole("status")).toHaveTextContent("Validando a integridade");

    act(() => {
      eventHandler?.({
        payload: {
          running: false,
          percent: 100,
          phase: "completed",
          message: "Backup concluído.",
        },
      });
    });
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("falls back to backend status when event registration fails", async () => {
    mockedListen.mockRejectedValue(new Error("event API unavailable"));
    mockedInvoke.mockResolvedValue({
      running: true,
      progressPercent: 15,
      phase: "checking",
    });

    const view = render(<AutomaticBackupProgress />);

    expect(await screen.findByRole("status")).toHaveTextContent("15% concluído");
    view.unmount();
  });
});
