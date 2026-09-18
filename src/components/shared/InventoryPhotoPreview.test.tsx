import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { InventoryPhotoPreview } from "@/components/shared/InventoryPhotoPreview";

describe("InventoryPhotoPreview", () => {
  it("opens an enlarged photo from the listing thumbnail", async () => {
    const user = userEvent.setup();
    render(<InventoryPhotoPreview itemName="Capa azul" photoDataUrl="data:image/png;base64,iVBORw0KGgo=" />);

    await user.click(screen.getByRole("button", { name: "Ampliar foto de Capa azul" }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Foto ampliada de Capa azul" })).toHaveAttribute("src", "data:image/png;base64,iVBORw0KGgo=");
  });
});
