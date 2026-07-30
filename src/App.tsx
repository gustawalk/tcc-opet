import { useEffect, useRef } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { Toaster } from "sonner";
import { toast } from "sonner";
import { check } from "@tauri-apps/plugin-updater";
import { MainLayout } from "./layouts/MainLayout";
import { Dashboard } from "./views/Dashboard";
import { ServiceOrderCreate } from "./views/ServiceOrderCreate";
import { Customers } from "./views/Customers";
import { Inventory } from "./views/Inventory";
import { ServiceOrders } from "./views/ServiceOrders";
import { Users } from "./views/Users";
import { Settings } from "./views/Settings";
import { Templates } from "./views/Templates";
import { Reports } from "./views/Reports";
import { ServiceOrderDrawerProvider } from "./components/shared/ServiceOrderDrawerProvider";
import { CustomerDrawerProvider } from "./components/shared/CustomerDrawerProvider";

// Create a client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      retry: 1,
    },
  },
});

function UpdateAvailabilityNotice() {
  const hasChecked = useRef(false);

  useEffect(() => {
    if (import.meta.env.DEV || hasChecked.current) return;
    hasChecked.current = true;

    void check()
      .then((update) => {
        if (!update) return;
        toast.info(`Atualização ${update.version} disponível. Verifique em Configurações > Atualizações.`);
      })
      .catch(() => undefined);
  }, []);

  return null;
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <UpdateAvailabilityNotice />
        <ServiceOrderDrawerProvider>
          <CustomerDrawerProvider>
            <MainLayout>
              <Routes>
                <Route path="/" element={<Dashboard />} />
                <Route path="/os" element={<ServiceOrders />} />
                <Route path="/os/new" element={<ServiceOrderCreate />} />
                <Route path="/customers" element={<Customers />} />
                <Route path="/inventory" element={<Inventory />} />
                <Route path="/templates" element={<Templates />} />
                <Route path="/reports" element={<Reports />} />
                <Route path="/users" element={<Users />} />
                <Route path="/settings" element={<Settings />} />
              </Routes>
            </MainLayout>
          </CustomerDrawerProvider>
        </ServiceOrderDrawerProvider>
      </BrowserRouter>
      <Toaster position="top-right" richColors closeButton duration={4000} />
      {import.meta.env.DEV && <ReactQueryDevtools initialIsOpen={false} />}
    </QueryClientProvider>
  );
}

export default App;
