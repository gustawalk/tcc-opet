import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Pagination } from "./Pagination";

afterEach(() => cleanup());

describe("Pagination", () => {
  it("shows the visible range and total", () => {
    render(
      <Pagination
        totalItems={47}
        page={1}
        pageSize={10}
        onPageChange={() => undefined}
        onPageSizeChange={() => undefined}
      />,
    );
    expect(screen.getByText(/Mostrando 1–10 de 47/)).toBeInTheDocument();
  });

  it("disables first/previous on the first page", () => {
    render(
      <Pagination
        totalItems={47}
        page={1}
        pageSize={10}
        onPageChange={() => undefined}
        onPageSizeChange={() => undefined}
      />,
    );
    expect(screen.getByRole("button", { name: "Primeira página" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Página anterior" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Próxima página" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Última página" })).toBeEnabled();
  });

  it("shows page numbers and reports clicks", async () => {
    const user = userEvent.setup();
    const onPageChange = vi.fn();
    render(
      <Pagination
        totalItems={47}
        page={3}
        pageSize={10}
        onPageChange={onPageChange}
        onPageSizeChange={() => undefined}
      />,
    );
    expect(screen.getByRole("button", { name: "3" })).toHaveClass("bg-primary");
    await user.click(screen.getByRole("button", { name: "2" }));
    expect(onPageChange).toHaveBeenCalledWith(2);
  });

  it("collapses the window with ellipses for many pages", () => {
    render(
      <Pagination
        totalItems={1000}
        page={1}
        pageSize={10}
        onPageChange={() => undefined}
        onPageSizeChange={() => undefined}
      />,
    );
    expect(screen.getByText("…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "100" })).toBeInTheDocument();
  });

  it("reports page size changes", async () => {
    const user = userEvent.setup();
    const onPageSizeChange = vi.fn();
    render(
      <Pagination
        totalItems={47}
        page={1}
        pageSize={10}
        onPageChange={() => undefined}
        onPageSizeChange={onPageSizeChange}
      />,
    );
    await user.selectOptions(screen.getByLabelText("Itens por página"), "50");
    expect(onPageSizeChange).toHaveBeenCalledWith(50);
  });

  it("scrolls to the list when pagination changes", async () => {
    const user = userEvent.setup();
    const scrollIntoView = vi.fn();
    const target = document.createElement("div");
    target.scrollIntoView = scrollIntoView;
    const scrollTargetRef = { current: target };

    render(
      <Pagination
        totalItems={47}
        page={1}
        pageSize={10}
        onPageChange={() => undefined}
        onPageSizeChange={() => undefined}
        scrollTargetRef={scrollTargetRef}
      />,
    );

    await user.click(screen.getByRole("button", { name: "2" }));
    await user.selectOptions(screen.getByLabelText("Itens por página"), "50");

    expect(scrollIntoView).toHaveBeenCalledTimes(2);
    expect(scrollIntoView).toHaveBeenLastCalledWith({
      behavior: "smooth",
      block: "start",
    });
  });
});
