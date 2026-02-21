# Plano de Projeto: Sistema de Gerenciamento de Ordens de Serviço (Versão Arquitetura Desktop-First)

Este documento detalha a estrutura de desenvolvimento do sistema multiplataforma (TCC), utilizando **Tauri v2**, **Vite**, React e Rust, focado em operação **Local-First**, rastreabilidade e alta confiabilidade.

---

# 1. Visão Geral da Arquitetura

O sistema segue o modelo **Local-First** com SQLite como fonte primária de verdade.

## Stack

* **Frontend:** React + TypeScript + Vite + Tailwind + shadcn/ui
* **Roteamento:** React Router (SPA)
* **Estado e Cache:** TanStack Query (sincronização com backend via `invoke`)
* **Backend:** Rust (Tauri) responsável por:

  * Persistência SQLite
  * Sistema de arquivos
  * Geração de PDF
  * Regras críticas de negócio
* **Comunicação:** IPC assíncrono via `invoke`, retornando `Result<T, E>` padronizado

## Princípios Arquiteturais

* SQLite é a fonte da verdade
* Nenhuma regra crítica fica no frontend
* Toda mutação de banco usa transação
* Estoque nunca é alterado diretamente (apenas via movimentação)

---

# 2. Requisitos Funcionais

## 2.1 Base do Sistema

* **RF001:** Cadastro de Clientes
* **RF002:** Abertura de Ordem de Serviço
* **RF003:** Fluxo de Status controlado (Orçamento → Manutenção → Aguardando Peça → Finalizada)
* **RF004:** Consulta de Status Local (modo cliente)
* **RF005:** Geração de PDF da OS
* **RF006:** Gestão de Usuários (Admin / Técnico)
* **RF007:** Preparação para sincronização futura (modo híbrido opcional)

---

## 2.2 Gestão de Estoque e Evidências

* **RF008:** Controle de Estoque com:

  * Quantidade mínima
  * Preço de custo
  * Preço de venda
* **RF009:** Assinatura Digital (Canvas → salva como imagem local)
* **RF010:** Upload de fotos (antes/depois) via API de arquivos do Tauri

---

## 2.3 Inteligência Financeira

* **RF011:** Dashboard Financeiro com:

  * Faturamento bruto
  * Custo total
  * Lucro real calculado
* **RF012:** Soft Deletes para integridade histórica

---

# 3. Requisitos Não Funcionais

* **RNF001:** 100% Offline
* **RNF002:** Hash de senha com Argon2 no backend Rust
* **RNF003:** Build nativo Windows/Linux
* **RNF004:** UI Responsiva
* **RNF005:** Inicialização < 2s
* **RNF006:** Migrações versionadas via `PRAGMA user_version`
* **RNF007:** Integridade transacional obrigatória

---

# 4. Detalhamento Técnico

## 4.1 Banco de Dados (SQLite Revisado)

### settings

```
id
company_name
cnpj
logo_path
address
```

### users

```
id
name
email UNIQUE
password_hash
role (admin | tech)
created_at
deleted_at
```

### customers

```
id
name
phone
email
address
created_at
deleted_at
```

### inventory

```
id
name
description
min_quantity
cost_price
sale_price
created_at
deleted_at
```

⚠️ `quantity` removido daqui.

---

### inventory_movements (NOVO — obrigatório)

```
id
product_id
type (entrada | saida)
quantity
reference_os_id NULLABLE
created_at
```

Estoque é calculado por soma de movimentos.

---

### service_orders

```
id
customer_id
equipment
description
status
signature_path
created_at
updated_at
closed_at NULLABLE
```

⚠️ `total_price` removido como campo confiável.
Ele será:

* Calculado dinamicamente
* Ou congelado no fechamento da OS

---

### os_items

```
id
os_id
product_id
quantity
unit_cost_snapshot
unit_price_snapshot
```

Snapshot congela valores no momento da venda.

---

### os_photos

```
id
os_id
file_path
category (before | after)
created_at
```

---

# 4.2 Backend Commands (Revisado)

* `db_migrate`
* `get_customers`
* `upsert_customer`
* `get_inventory`
* `create_inventory_movement`
* `create_os`
* `add_item_to_os`
* `update_os_status`
* `close_os`
* `get_financial_summary`
* `generate_os_pdf`

Todas mutações usam transações.

---

# 4.3 Estrutura de Pastas (Frontend - Vite)

```
src/
 ├─ main.tsx
 ├─ router.tsx
 ├─ queries/
 ├─ components/
 │   ├─ ui/
 │   └─ shared/
 ├─ pages/
 │   ├─ dashboard/
 │   ├─ customers/
 │   ├─ inventory/
 │   ├─ service-orders/
 │   ├─ users/
 │   └─ settings/
 ├─ lib/
 └─ types/
```

Roteamento via React Router.

Sem sistema baseado em pasta automática.

---

# 5. Sincronização Híbrida (Estratégia Definida)

Modo futuro opcional:

* SQLite continua sendo a fonte primária.
* Sync funciona como replicação eventual.
* Estratégia inicial: Last-Write-Wins via `updated_at`.
* Cada registro terá:

  * `updated_at`
  * `device_id` (para rastreio)

Sem CRDT na primeira versão.

---

# 6. Fases de Implementação

## Fase 1 – Core

* Setup Vite + Tauri
* Setup Tailwind
* Setup React Router
* Setup TanStack Query
* Implementar migrations versionadas
* Criar schema base

---

## Fase 2 – CRUD Base

* CRUD Clientes
* CRUD Inventário (via movements)
* Sistema de autenticação (Argon2)

---

## Fase 3 – Motor de OS

* Criar OS
* Adicionar itens
* Movimentar estoque automaticamente
* Encerrar OS (snapshot financeiro)

---

## Fase 4 – Evidências e PDF

* Assinatura digital
* Upload de fotos
* Gerador de PDF
* Persistência de caminho

---

## Fase 5 – Dashboard Financeiro

* Query agregada via SQL
* Cálculo de lucro real
* Alertas de estoque mínimo

---

# 7. Convenções de Engenharia

* Toda mutação crítica usa transação
* Nenhum cálculo financeiro fica só no frontend
* Tipos Rust sincronizados com TypeScript
* Estoque sempre via `inventory_movements`
* Nunca deletar fisicamente registros críticos

---

# 8. Views do Sistema (SPA)

* Dashboard
* Clientes
* Estoque
* Ordens de Serviço
* Editor de OS
* Usuários
* Configurações

Consulta externa será:

* Um modo simplificado dentro do app
* Não depende de servidor web
