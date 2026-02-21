import { BrowserRouter, Routes, Route } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { MainLayout } from "./layouts/MainLayout";
import { Dashboard } from "./views/Dashboard";
import { ServiceOrderCreate } from "./views/ServiceOrderCreate";
import { Customers } from "./views/Customers";
import { Inventory } from "./views/Inventory";
import { ServiceOrders } from "./views/ServiceOrders";
import { Users } from "./views/Users";
import { Settings } from "./views/Settings";

// Create a client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      retry: 1,
    },
  },
});

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <MainLayout>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/os" element={<ServiceOrders />} />
            <Route path="/os/new" element={<ServiceOrderCreate />} />
            <Route path="/customers" element={<Customers />} />
            <Route path="/inventory" element={<Inventory />} />
            <Route path="/users" element={<Users />} />
            <Route path="/tracking" element={<div className="p-8">Consulta Pública (Em breve)</div>} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </MainLayout>
      </BrowserRouter>
      <ReactQueryDevtools initialIsOpen={false} />
    </QueryClientProvider>
  );
}

export default App;
