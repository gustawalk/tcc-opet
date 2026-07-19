export type OSStatus = "Orçamento" | "Em Manutenção" | "Aguardando Peça" | "Finalizada" | "Cancelada";

export interface Customer {
  id: string;
  name: string;
  phone: string;
  email: string;
  address: string;
  createdAt?: string;
  deletedAt?: string | null;
}

export interface User {
  id: string;
  name: string;
  email: string;
  phone?: string;
  cpf?: string;
  joinDate?: string;
  createdAt?: string;
  deletedAt?: string | null;
}

export interface InventoryItem {
  id: string;
  name: string;
  description: string;
  type: 'part' | 'service';
  minQuantity: number;
  currentQuantity: number;
  costPrice: number;
  averageCost: number;
  salePrice: number;
  supplierName?: string | null;
  createdAt?: string;
  updatedAt?: string;
  deletedAt?: string | null;
}

export interface ServiceOrder {
  id: string;
  customerId: string;
  customerName?: string;
  userId?: string | null;
  userName?: string | null;
  equipment: string;
  imei?: string;
  description: string;
  status: OSStatus;
  totalPrice?: number;
  createdAt: string;
  updatedAt?: string;
  closedAt?: string | null;
  displayId: string;
  discountPercent: number;
}

export interface Settings {
  id?: number;
  companyName: string;
  cnpj: string;
  logoPath?: string;
  address: string;
}

export interface SystemInfo {
  databasePath: string;
  appVersion: string;
  tauriVersion: string;
  environment: string;
}

export interface UpdateCheck {
  configured: boolean;
  currentVersion: string;
  latestVersion?: string | null;
  updateAvailable: boolean;
  downloadUrl?: string | null;
}

export interface BackupSummary {
  path: string;
  attachmentCount: number;
}

export interface ChecklistItem {
  id: string;
  label: string;
  checked: boolean;
}

export interface ServiceOrderEvent {
  id: string;
  serviceOrderId: string;
  eventType: string;
  details: string;
  createdAt: string;
}

export interface ServiceOrderAttachment {
  id: string;
  serviceOrderId: string;
  fileName: string;
  storageName: string;
  mimeType: string;
  sizeBytes: number;
  createdAt: string;
}

export interface ChecklistTemplate {
  id: string;
  title: string;
  items: string[];
  createdAt?: string;
}

export interface OSChecklist {
  title: string;
  items: ChecklistItem[];
}

export interface ServiceOrderPart {
  id: string;
  serviceOrderId: string;
  inventoryItemId: string;
  inventoryItemName: string;
  itemType: 'part' | 'service';
  currentQuantity: number;
  quantity: number;
  unitCost: number;
  unitPrice: number;
}

export interface FinancialSummary {
  totalRevenue: number;
  netProfit: number;
  partsInUseCost: number;
  activeOrdersCount: number;
  revenueTrend: { value: string; isPositive: boolean };
  profitTrend: { value: string; isPositive: boolean };
}

export interface RecentOS {
  id: string;
  customerName: string;
  equipment: string;
  status: OSStatus;
  createdAt: string;
  totalPrice: number;
  displayId: string;
  discountPercent: number;
}

export interface InventoryMovement {
  id: string;
  inventoryItemId: string;
  type: 'entrada' | 'saida';
  quantity: number;
  referenceOsId?: string | null;
  osDisplayId?: string | null;
  reason: string;
  unitCost?: number | null;
  createdAt?: string;
}

export interface InactiveInventoryItem {
  id: string;
  name: string;
  currentQuantity: number;
  lastMovementAt?: string | null;
}

export interface AbcInventoryGroup {
  classification: "A" | "B" | "C";
  itemCount: number;
  inventoryValue: number;
}

export interface InventoryInsights {
  inactiveItems: InactiveInventoryItem[];
  abcGroups: AbcInventoryGroup[];
}

export interface InventoryAlert {
  id: string;
  name: string;
  currentStock: number;
  minStock: number;
}

export interface StatusCount {
  status: OSStatus;
  count: number;
}

export interface DashboardData {
  summary: FinancialSummary;
  recentOrders: RecentOS[];
  inventoryAlerts: InventoryAlert[];
  statusCounts: StatusCount[];
}

export interface FinancialBreakdown {
  label: string;
  revenue: number;
  cost: number;
  profit: number;
  count: number;
}

export interface FinancialMonth {
  month: string;
  revenue: number;
  profit: number;
  orderCount: number;
}

export interface FinancialReport {
  startDate: string;
  endDate: string;
  totalRevenue: number;
  totalCost: number;
  netProfit: number;
  averageTicket: number;
  finalizedOrders: number;
  byTechnician: FinancialBreakdown[];
  byItemType: FinancialBreakdown[];
  byMonth: FinancialMonth[];
}

export interface PdfPreview {
  token: string;
  dataUrl: string;
  fileName: string;
}
