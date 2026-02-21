export type OSStatus = "Orçamento" | "Em Manutenção" | "Aguardando Peça" | "Finalizada" | "Cancelada";

export interface Customer {
  id: string;
  name: string;
  phone: string;
  email: string;
  address: string;
  created_at?: string;
  deleted_at?: string | null;
}

export interface User {
  id: string;
  name: string;
  email: string;
  role: 'admin' | 'tech';
  created_at?: string;
  deleted_at?: string | null;
}

export interface InventoryItem {
  id: string;
  name: string;
  description: string;
  min_quantity: number;
  current_quantity: number;
  cost_price: number;
  sale_price: number;
  created_at?: string;
  deleted_at?: string | null;
}

export interface ServiceOrder {
  id: string;
  customer_id: string;
  customer_name?: string;
  equipment: string;
  description: string;
  status: OSStatus;
  total_price?: number;
  signature_path?: string | null;
  created_at: string;
  updated_at?: string;
  closed_at?: string | null;
}

export interface Settings {
  company_name: string;
  cnpj: string;
  logo_path?: string;
  address: string;
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
