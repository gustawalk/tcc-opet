import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EmployeeCreateSheet } from "@/components/shared/EmployeeCreateSheet";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@/lib/errors", () => ({ toastError: vi.fn(), toastSuccess: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

describe("EmployeeCreateSheet", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("returns the created employee so the order can select it automatically", async () => {
    const onCreated = vi.fn();
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    queryClient.setQueryData(["users"], [{ id: "existing", name: "Existente", email: "existing@example.com" }]);
    mockedInvoke.mockResolvedValue("employee-1");
    const user = userEvent.setup();

    render(
      <QueryClientProvider client={queryClient}>
        <EmployeeCreateSheet open onOpenChange={vi.fn()} onCreated={onCreated} />
      </QueryClientProvider>,
    );

    await user.type(screen.getByLabelText("Nome completo"), "Ana Técnica");
    await user.type(screen.getByLabelText("E-mail"), "ana@example.com");
    await user.click(screen.getByRole("button", { name: "Criar funcionário" }));

    await waitFor(() => {
      expect(onCreated).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "employee-1",
          name: "Ana Técnica",
          email: "ana@example.com",
        }),
      );
    });
    expect(mockedInvoke).toHaveBeenCalledWith("create_user", expect.any(Object));
    expect(queryClient.getQueryData<{ id: string }[]>(["users"])?.[0]?.id).toBe(
      "employee-1",
    );
  });
});
