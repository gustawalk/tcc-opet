import { useEffect, useRef, useState } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { Toaster } from "sonner";
import { toast } from "sonner";
import { check } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
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
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "./components/ui/dialog";
import { Button } from "./components/ui/button";

const UPDATE_PATCH_NOTES_STORAGE_KEY = "opets.pending-update-patch-notes";

type PendingPatchNotes = { version: string; body: string };

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

function UpdatePatchNotes() {
  const [patchNotes, setPatchNotes] = useState<PendingPatchNotes | null>(null);

  useEffect(() => {
    const stored = localStorage.getItem(UPDATE_PATCH_NOTES_STORAGE_KEY);
    if (!stored) return;

    let pending: PendingPatchNotes;
    try {
      pending = JSON.parse(stored);
    } catch {
      localStorage.removeItem(UPDATE_PATCH_NOTES_STORAGE_KEY);
      return;
    }
    if (
      typeof pending.version !== "string" ||
      typeof pending.body !== "string" ||
      !pending.body.trim()
    ) {
      localStorage.removeItem(UPDATE_PATCH_NOTES_STORAGE_KEY);
      return;
    }

    void getVersion()
      .then((version) => {
        if (version === pending.version.replace(/^v/, "")) {
          setPatchNotes(pending);
        } else {
          localStorage.removeItem(UPDATE_PATCH_NOTES_STORAGE_KEY);
        }
      })
      .catch(() => localStorage.removeItem(UPDATE_PATCH_NOTES_STORAGE_KEY));
  }, []);

  const close = () => {
    localStorage.removeItem(UPDATE_PATCH_NOTES_STORAGE_KEY);
    setPatchNotes(null);
  };

  return (
    <Dialog open={patchNotes !== null} onOpenChange={(open) => !open && close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Atualização concluída</DialogTitle>
          <DialogDescription>
            Você está usando a versão {patchNotes?.version}.
          </DialogDescription>
        </DialogHeader>
        <div className="max-h-72 overflow-y-auto whitespace-pre-line rounded-md border bg-muted/50 p-3 text-sm text-muted-foreground">
          {patchNotes?.body}
        </div>
        <DialogFooter>
          <Button type="button" onClick={close}>Fechar</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <UpdateAvailabilityNotice />
        <UpdatePatchNotes />
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
