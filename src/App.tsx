import { BrowserRouter, Routes, Route } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { MainLayout } from "./layouts/MainLayout";
import { Dashboard } from "./views/Dashboard";
import { ServiceOrderCreate } from "./views/ServiceOrderCreate";

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
            <Route path="/os" element={<div className="p-8">Visualização de OS (Em breve)</div>} />
            <Route path="/os/new" element={<ServiceOrderCreate />} />
            <Route path="/customers" element={<div className="p-8">Visualização de Clientes (Em breve)</div>} />
            <Route path="/inventory" element={<div className="p-8">Visualização de Estoque (Em breve)</div>} />
            <Route path="/tracking" element={<div className="p-8">Consulta Pública (Em breve)</div>} />
            <Route path="/settings" element={<div className="p-8">Configurações (Em breve)</div>} />
          </Routes>
        </MainLayout>
      </BrowserRouter>
      <ReactQueryDevtools initialIsOpen={false} />
    </QueryClientProvider>
  );
}

export default App;
