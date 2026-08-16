import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RelativeDate } from "@/components/shared/RelativeDate";

const TIME_OPTIONS: Intl.DateTimeFormatOptions = { hour12: false };

describe("RelativeDate", () => {
  afterEach(cleanup);

  const now = new Date("2026-08-16T12:00:00");

  it("shows the fallback when there is no value", () => {
    render(<RelativeDate value={null} now={now} />);
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("formats today as Hoje with local time", () => {
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 6, 16, 43);
    render(<RelativeDate value={today.toISOString()} now={now} />);
    const expected = `Hoje ${today.toLocaleTimeString("pt-BR", TIME_OPTIONS)}`;
    expect(screen.getByText(expected)).toBeInTheDocument();
  });

  it("formats yesterday as Ontem with local time", () => {
    const yesterday = new Date(now.getTime() - 24 * 60 * 60 * 1000);
    render(<RelativeDate value={yesterday.toISOString()} now={now} />);
    const expected = `Ontem ${yesterday.toLocaleTimeString("pt-BR", TIME_OPTIONS)}`;
    expect(screen.getByText(expected)).toBeInTheDocument();
  });

  it("keeps the full formatted date for older dates", () => {
    const older = new Date(now.getTime() - 10 * 24 * 60 * 60 * 1000);
    render(<RelativeDate value={older.toISOString()} now={now} />);
    expect(screen.getByText(older.toLocaleString("pt-BR"))).toBeInTheDocument();
  });

  it("uses the custom fallback passed by the caller", () => {
    render(
      <RelativeDate value={undefined} fallback="Ainda não executado" now={now} />,
    );
    expect(screen.getByText("Ainda não executado")).toBeInTheDocument();
  });
});