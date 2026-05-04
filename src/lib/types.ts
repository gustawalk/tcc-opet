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
  role: 'admin' | 'tech';
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
  salePrice: number;
  createdAt?: string;
  deletedAt?: string | null;
}

export interface ServiceOrder {
  id: string;
  customerId: string;
  customerName?: string;
  userId?: string | null;
  equipment: string;
  imei?: string;
  description: string;
  status: OSStatus;
  totalPrice?: number;
  signaturePath?: string | null;
  createdAt: string;
  updatedAt?: string;
  closedAt?: string | null;
}

export interface Settings {
  companyName: string;
  cnpj: string;
  logoPath?: string;
  address: string;
}

export interface ChecklistItem {
  id: string;
  label: string;
  checked: boolean;
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
