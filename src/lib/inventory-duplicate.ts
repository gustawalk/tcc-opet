import { InventoryItem } from "@/lib/types";

export type DuplicateInventoryPayload = Pick<
  InventoryItem,
  | "name"
  | "description"
  | "type"
  | "minQuantity"
  | "costPrice"
  | "salePrice"
> & {
  supplierName: string;
  initialQuantity: number;
};

export function isUnchangedDuplicate(
  data: DuplicateInventoryPayload,
  original: InventoryItem,
) {
  return (
    data.name.trim() === original.name.trim() &&
    data.description.trim() === original.description.trim() &&
    data.type === original.type &&
    data.supplierName.trim() === (original.supplierName ?? "").trim() &&
    data.minQuantity === original.minQuantity &&
    data.initialQuantity === 0 &&
    data.costPrice === original.costPrice &&
    data.salePrice === original.salePrice
  );
}
