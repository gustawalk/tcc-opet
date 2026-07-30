import { createContext, ReactNode, useContext, useState } from "react";
import { CustomerHistorySheet } from "@/components/shared/CustomerHistorySheet";

interface CustomerDrawerContextValue {
  openCustomerHistory: (id: string) => void;
  closeCustomerHistory: () => void;
}

const CustomerDrawerContext = createContext<CustomerDrawerContextValue | null>(null);

export function CustomerDrawerProvider({ children }: { children: ReactNode }) {
  const [customerId, setCustomerId] = useState<string | null>(null);
  const closeCustomerHistory = () => setCustomerId(null);

  return (
    <CustomerDrawerContext.Provider
      value={{
        openCustomerHistory: setCustomerId,
        closeCustomerHistory,
      }}
    >
      {children}
      <CustomerHistorySheet
        customerId={customerId}
        open={customerId !== null}
        onClose={closeCustomerHistory}
      />
    </CustomerDrawerContext.Provider>
  );
}

export function useCustomerDrawer() {
  const context = useContext(CustomerDrawerContext);
  if (!context) {
    throw new Error("useCustomerDrawer must be used within CustomerDrawerProvider");
  }
  return context;
}
