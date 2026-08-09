import { describe, expect, it } from "vitest";
import { prioritizeInventoryAlerts } from "@/views/Dashboard";

describe("prioritizeInventoryAlerts", () => {
  it("puts out-of-stock alerts before below-minimum alerts", () => {
    const alerts = prioritizeInventoryAlerts([
      { id: "yellow-high", name: "Bateria", currentStock: 4, minStock: 5 },
      { id: "red", name: "Tela", currentStock: 0, minStock: 2 },
      { id: "yellow-low", name: "Cabo", currentStock: 1, minStock: 5 },
    ]);

    expect(alerts.map((alert) => alert.id)).toEqual([
      "red",
      "yellow-low",
      "yellow-high",
    ]);
  });

  it("keeps an empty alert list empty", () => {
    expect(prioritizeInventoryAlerts([])).toEqual([]);
  });
});
