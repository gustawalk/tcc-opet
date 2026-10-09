import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ReleaseNotesMarkdown } from "./ReleaseNotesMarkdown";

describe("ReleaseNotesMarkdown", () => {
  afterEach(cleanup);
  it("renders release headings, lists, links, and emphasis", () => {
    render(<ReleaseNotesMarkdown body={"## Novidades\n\n- **Busca** melhorada\n- [Detalhes](https://example.com/release)"} />);

    expect(screen.getByRole("heading", { name: "Novidades" })).toBeInTheDocument();
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    expect(screen.getByText("Busca").tagName).toBe("STRONG");
    expect(screen.getByRole("link", { name: "Detalhes" })).toHaveAttribute("href", "https://example.com/release");
  });

  it("does not render HTML or unsafe links from update notes", () => {
    render(<ReleaseNotesMarkdown body={"<script>alert(1)</script>\n\n[Run](javascript:alert(1))"} />);

    expect(document.querySelector("script")).toBeNull();
    expect(screen.getByText("Run").closest("a")).toHaveAttribute("href", "");
  });
});
