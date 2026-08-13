import {
  createContext,
  lazy,
  ReactNode,
  Suspense,
  useContext,
  useState,
} from "react";

const CustomerHistorySheet = lazy(() =>
  import("@/components/shared/CustomerHistorySheet").then(
    ({ CustomerHistorySheet }) => ({ default: CustomerHistorySheet }),
  ),
);

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
      {customerId && (
        <Suspense fallback={null}>
          <CustomerHistorySheet
            customerId={customerId}
            open
            onClose={closeCustomerHistory}
          />
        </Suspense>
      )}
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
