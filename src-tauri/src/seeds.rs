use crate::database::get_db;
use chrono::{Duration, Utc};
use rusqlite::params;
use uuid::Uuid;

/// Initialize seed data - seeds missing tables independently
pub fn initialize_seed_data() -> Result<(), String> {
    let conn = get_db().map_err(|e| e.to_string())?;

    println!("[SEED] Checking seed data requirements...");

    // Seed users first (needed for service orders)
    let user_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .unwrap_or(0);
    println!("[SEED] Users count: {}", user_count);
    if user_count == 0 {
        println!("[SEED] Seeding users...");
        seed_users(&conn)?;
        println!("[SEED] Users seeded successfully");
    } else {
        println!("[SEED] Users already exist, skipping");
    }

    // Seed customers
    let customer_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM customers", [], |row| row.get(0))
        .unwrap_or(0);
    println!("[SEED] Customers count: {}", customer_count);
    if customer_count == 0 {
        println!("[SEED] Seeding customers...");
        seed_customers(&conn)?;
        println!("[SEED] Customers seeded successfully");
    } else {
        println!("[SEED] Customers already exist, skipping");
    }

    // Seed inventory (needed for service orders)
    let inventory_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM inventory_items", [], |row| row.get(0))
        .unwrap_or(0);
    println!("[SEED] Inventory count: {}", inventory_count);
    if inventory_count == 0 {
        println!("[SEED] Seeding inventory...");
        seed_inventory(&conn)?;
        println!("[SEED] Inventory seeded successfully");
    } else {
        println!("[SEED] Inventory already exists, skipping");
    }

    // Seed service orders (needs users, customers, and inventory)
    let order_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM service_orders", [], |row| row.get(0))
        .unwrap_or(0);
    println!("[SEED] Service orders count: {}", order_count);
    if order_count == 0 {
        println!("[SEED] About to seed service_orders...");
        seed_service_orders(&conn)?;
        
        // Seed financial snapshots only if empty (migration might have inserted one for today)
        let financial_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM financial_snapshots", [], |row| row.get(0))
            .unwrap_or(0);
        println!("[SEED] Financial snapshots count: {}", financial_count);
        if financial_count == 0 {
            println!("[SEED] About to seed financial_snapshots...");
            seed_financial_snapshots(&conn)?;
        } else {
            println!("[SEED] Financial snapshots already exist, skipping");
        }
        
        println!("[SEED] Service orders seeded successfully");
    } else {
        println!("[SEED] Service orders already exist, skipping");
    }

    // Always seed checklist templates (in case they were added after initial seed)
    let template_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM checklist_templates", [], |row| row.get(0))
        .unwrap_or(0);
    println!("[SEED] Checklist templates count: {}", template_count);
    if template_count == 0 {
        println!("[SEED] About to seed checklist templates...");
        seed_checklist_templates(&conn)?;
        println!("[SEED] Checklist templates seeded successfully");
    } else {
        println!("[SEED] Checklist templates already exist, skipping");
    }

    println!("[SEED] Seed initialization complete!");
    Ok(())
}

fn seed_users(conn: &rusqlite::Connection) -> Result<(), String> {
    let users = vec![
        ("Gustavo Admin", "admin@opet.com.br", "admin"),
        ("João Técnico", "joao@opet.com.br", "tech"),
        ("Maria Técnica", "maria@opet.com.br", "tech"),
    ];

    let now = Utc::now().to_rfc3339();

    for (name, email, role) in users {
        conn.execute(
            "INSERT INTO users (id, name, email, password, role, created_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
            params![Uuid::new_v4().to_string(), name, email, "123456", role, now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn seed_customers(conn: &rusqlite::Connection) -> Result<(), String> {
    let customers = vec![
        (
            "Maria Silva",
            "(41) 99999-1111",
            "maria@email.com",
            "Rua das Flores, 123 - Curitiba",
        ),
        (
            "João Pereira",
            "(41) 98888-2222",
            "joao@email.com",
            "Av. Principal, 500 - Araucária",
        ),
        (
            "Empresa ABC",
            "(41) 3333-4444",
            "contato@abc.com",
            "Rua Industrial, 10 - Curitiba",
        ),
        (
            "Carlos Oliveira",
            "(41) 97777-3333",
            "carlos@email.com",
            "Av. Brasil, 250 - Curitiba",
        ),
        (
            "Tech Solutions Corp",
            "(41) 3001-5555",
            "info@techcorp.com",
            "Centro Empresarial, 100 - Curitiba",
        ),
        (
            "Ana Costa",
            "(41) 99222-4444",
            "ana@email.com",
            "Rua das Acácias, 75 - Pinhais",
        ),
        (
            "Roberto Mendes",
            "(41) 98111-6666",
            "roberto@email.com",
            "Av. Paraná, 500 - Curitiba",
        ),
        (
            "Startup XYZ",
            "(41) 3221-7777",
            "contact@startupxyz.com",
            "Polo Tech, 200 - Curitiba",
        ),
        (
            "Paula Santos",
            "(41) 99333-8888",
            "paula@email.com",
            "Rua Santa Catarina, 88 - Curitiba",
        ),
        (
            "IT Services Ltd",
            "(41) 3333-9999",
            "admin@itservices.com",
            "Edifício Commerce, 500 - Curitiba",
        ),
    ];

    let now = Utc::now().to_rfc3339();

    for (name, phone, email, address) in customers {
        conn.execute(
            "INSERT INTO customers (id, name, phone, email, address, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL)",
            params![Uuid::new_v4().to_string(), name, phone, email, address, now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn seed_inventory(conn: &rusqlite::Connection) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();

    // Parts - with some low stock for alerts
    let parts = vec![
        (
            "Tela iPhone 12",
            "Tela de reposição para iPhone 12",
            "part",
            5,
            3,
            80.00,
            180.00,
        ),
        (
            "Bateria Samsung Galaxy",
            "Bateria de lítio para Samsung",
            "part",
            5,
            8,
            45.00,
            120.00,
        ),
        (
            "Carregador USB-C",
            "Carregador universal USB-C",
            "part",
            10,
            2,
            12.00,
            45.00,
        ),
        (
            "Pasta Térmica Arctic Silver",
            "Pasta térmica premium",
            "part",
            5,
            0,
            8.00,
            35.00,
        ),
        (
            "Conector de Carga iPhone 13",
            "Conector Lightning para iPhone 13",
            "part",
            5,
            12,
            15.00,
            55.00,
        ),
        (
            "Teclado Notebook Acer",
            "Teclado de reposição Acer",
            "part",
            2,
            4,
            55.00,
            150.00,
        ),
        ("Webcam HD", "Webcam 1080p USB", "part", 3, 6, 35.00, 100.00),
        (
            "SSD 240GB",
            "Unidade SSD 240GB",
            "part",
            3,
            9,
            70.00,
            180.00,
        ),
        (
            "RAM DDR4 8GB",
            "Memória RAM 8GB DDR4",
            "part",
            3,
            7,
            65.00,
            160.00,
        ),
        (
            "Placa Mãe",
            "Placa mãe genérica",
            "part",
            1,
            2,
            120.00,
            300.00,
        ),
    ];

    for (name, description, item_type, min_qty, current_qty, cost, sale) in parts {
        conn.execute(
            "INSERT INTO inventory_items (id, name, description, type, min_quantity, current_quantity, cost_price, sale_price, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL)",
            params![
                Uuid::new_v4().to_string(),
                name,
                description,
                item_type,
                min_qty,
                current_qty,
                cost,
                sale,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    // Services
    let services = vec![
        (
            "Mão de Obra iPhone",
            "Troca de componentes básicos em iPhone",
            "service",
            0,
            999,
            50.00,
            150.00,
        ),
        (
            "Mão de Obra Notebook",
            "Reparo estrutural ou eletrônico em notebooks",
            "service",
            0,
            999,
            60.00,
            180.00,
        ),
        (
            "Limpeza Preventiva",
            "Limpeza e troca de pasta térmica",
            "service",
            0,
            999,
            10.00,
            80.00,
        ),
        (
            "Diagnóstico",
            "Diagnóstico completo do equipamento",
            "service",
            0,
            999,
            0.00,
            50.00,
        ),
        (
            "Instalação de Software",
            "Instalação e configuração de softwares",
            "service",
            0,
            999,
            0.00,
            60.00,
        ),
    ];

    for (name, description, item_type, min_qty, current_qty, cost, sale) in services {
        conn.execute(
            "INSERT INTO inventory_items (id, name, description, type, min_quantity, current_quantity, cost_price, sale_price, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL)",
            params![
                Uuid::new_v4().to_string(),
                name,
                description,
                item_type,
                min_qty,
                current_qty,
                cost,
                sale,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn seed_service_orders(conn: &rusqlite::Connection) -> Result<(), String> {
    println!("[SEED] seed_service_orders: Starting...");

    // Get all customers, users, and inventory items
    println!("[SEED] seed_service_orders: Fetching customers...");
    let mut customer_stmt = conn
        .prepare("SELECT id FROM customers ORDER BY ROWID")
        .map_err(|e| e.to_string())?;
    let customers: Vec<String> = customer_stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| e.to_string())?;
    println!("[SEED] seed_service_orders: Found {} customers", customers.len());

    println!("[SEED] seed_service_orders: Fetching users...");
    let mut user_stmt = conn
        .prepare("SELECT id FROM users WHERE role = 'tech'")
        .map_err(|e| e.to_string())?;
    let users: Vec<String> = user_stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| e.to_string())?;
    println!("[SEED] seed_service_orders: Found {} tech users", users.len());

    println!("[SEED] seed_service_orders: Fetching inventory items...");
    let mut inventory_stmt = conn
        .prepare("SELECT id, sale_price, cost_price FROM inventory_items ORDER BY ROWID")
        .map_err(|e| e.to_string())?;
    let inventory_items: Vec<(String, f64, f64)> = inventory_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    println!("[SEED] seed_service_orders: Found {} inventory items", inventory_items.len());

    let equipment_types = vec![
        "iPhone 13 Pro",
        "Notebook Dell G15",
        "Impressora HP LaserJet",
        "Samsung Galaxy S21",
        "iPad Air",
        "Drone DJI Mini",
        "Monitor LG 24\"",
        "Impressora Epson",
        "Webcam Logitech",
        "Roteador Wi-Fi",
    ];

    let statuses = vec![
        ("Finalizada", true), // 40% - 12 orders
        ("Finalizada", true),
        ("Finalizada", true),
        ("Finalizada", true),
        ("Finalizada", true),
        ("Finalizada", true),
        ("Finalizada", true),
        ("Finalizada", true),
        ("Finalizada", true),
        ("Finalizada", true),
        ("Finalizada", true),
        ("Finalizada", true),
        ("Em Manutenção", false), // 35% - 10 orders
        ("Em Manutenção", false),
        ("Em Manutenção", false),
        ("Em Manutenção", false),
        ("Em Manutenção", false),
        ("Em Manutenção", false),
        ("Em Manutenção", false),
        ("Em Manutenção", false),
        ("Em Manutenção", false),
        ("Em Manutenção", false),
        ("Aguardando Peça", false), // 15% - 4 orders
        ("Aguardando Peça", false),
        ("Aguardando Peça", false),
        ("Aguardando Peça", false),
        ("Orçamento", false), // 5% - 1 order
        ("Cancelada", false), // 5% - 1 order
    ];

    let descriptions = vec![
        "Troca de tela danificada",
        "Reparo de bateria com mau funcionamento",
        "Limpeza preventiva e reinstalação do SO",
        "Reparo de conector de carga",
        "Substituição de teclado com falha",
        "Atualização de BIOS e drivers",
        "Diagnóstico completo de falhas",
        "Troca de pasta térmica e limpeza",
    ];

    let now = Utc::now();

    println!("[SEED] seed_service_orders: Creating {} service orders...", statuses.len());

    for (idx, (status, is_finished)) in statuses.iter().enumerate() {
        println!("[SEED] seed_service_orders: Creating order {}/{}", idx + 1, statuses.len());
        
        let customer_id = &customers[idx % customers.len()];
        let user_id = &users[idx % users.len()];
        let equipment = equipment_types[idx % equipment_types.len()];
        let description = descriptions[idx % descriptions.len()];

        // Spread orders across 30 days
        let days_ago = 30 - (idx as i64 % 30);
        let created_at = (now - Duration::days(days_ago)).to_rfc3339();

        let closed_at = if *is_finished {
            Some((now - Duration::days(days_ago - 2)).to_rfc3339())
        } else {
            None
        };

        let order_id = Uuid::new_v4().to_string();
        let imei = if equipment.contains("iPhone")
            || equipment.contains("Galaxy")
            || equipment.contains("iPad")
        {
            Some(format!("{}123456789012", idx))
        } else {
            None
        };

        // Insert service order
        conn.execute(
            "INSERT INTO service_orders (id, customer_id, customer_name, user_id, equipment, imei, description, status, total_price, signature_path, created_at, updated_at, closed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?11, ?12)",
            params![
                order_id,
                customer_id,
                None::<String>,
                user_id,
                equipment,
                imei,
                description,
                status,
                0.0,
                created_at,
                created_at,
                closed_at
            ],
        )
        .map_err(|e| e.to_string())?;

        // Add 2-4 items to the order
        let num_items = 2 + (idx % 3);
        let mut total_price = 0.0;

        for item_idx in 0..num_items {
            let (inventory_id, sale_price, cost_price) =
                &inventory_items[(idx * 3 + item_idx) % inventory_items.len()];

            let quantity = 1;
            total_price += sale_price;

            conn.execute(
                "INSERT INTO service_order_parts (id, service_order_id, inventory_item_id, quantity, unit_cost, unit_price)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    Uuid::new_v4().to_string(),
                    order_id,
                    inventory_id,
                    quantity,
                    cost_price,
                    sale_price
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        // Update order with calculated total_price
        conn.execute(
            "UPDATE service_orders SET total_price = ?1 WHERE id = ?2",
            params![total_price, order_id],
        )
        .map_err(|e| e.to_string())?;
    }

    println!("[SEED] seed_service_orders: All orders created, adding checklists...");

    // Add checklists to some "Finalizada" orders for demonstration
    // Get the first 6 service orders that were created (these are the "Finalizada" ones)
    let mut os_stmt = conn
        .prepare("SELECT id, equipment FROM service_orders WHERE status = 'Finalizada' LIMIT 6")
        .map_err(|e| e.to_string())?;
    let orders_with_checklist: Vec<(String, String)> = os_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<(String, String)>, _>>()
        .map_err(|e| e.to_string())?;

    // Define checklist items based on equipment type
    let smartphone_items = vec![
        ("Tela/Touch", true),
        ("Câmeras", true),
        ("Microfone/Áudio", true),
        ("Carregamento", false),
        ("Botões", true),
        ("Wi-Fi/Bluetooth", true),
        ("Sensor de proximidade", true),
        ("Alto-falante", false),
    ];

    let notebook_items = vec![
        ("Teclado", true),
        ("Touchpad", true),
        ("Tela/Monitor", true),
        ("Webcam", false),
        ("Portas USB", true),
        ("Carregador", true),
        ("Bateria", true),
        ("Ventilação", false),
    ];

    let printer_items = vec![
        ("Alimentação de papel", true),
        ("Impressão", true),
        ("Scanner", false),
        ("Conexão USB", true),
        ("Conexão Wi-Fi", false),
        ("Roletes", true),
        ("Display/Led", true),
    ];

    for (order_id, equipment) in orders_with_checklist {
        let items = if equipment.contains("iPhone") || equipment.contains("Galaxy") || equipment.contains("iPad") || equipment.contains("Samsung") {
            &smartphone_items
        } else if equipment.contains("Notebook") || equipment.contains("Monitor") {
            &notebook_items
        } else if equipment.contains("Impressora") {
            &printer_items
        } else {
            &smartphone_items
        };

        for (label, checked) in items {
            conn.execute(
                "INSERT INTO service_order_checklists (id, service_order_id, label, checked)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    Uuid::new_v4().to_string(),
                    order_id,
                    label,
                    checked
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    println!("[SEED] seed_service_orders: Completed successfully!");
    Ok(())
}

fn seed_financial_snapshots(conn: &rusqlite::Connection) -> Result<(), String> {
    println!("[SEED] seed_financial_snapshots: Starting...");
    let now = Utc::now();

    // Create 30 daily snapshots
    for day_offset in 0..30 {
        println!("[SEED] seed_financial_snapshots: Processing day {}", day_offset);
        let snapshot_date = (now - Duration::days(day_offset as i64))
            .format("%Y-%m-%d")
            .to_string();

        // Calculate financial data for this day
        let query = "
            SELECT
                COALESCE(SUM(CASE WHEN so.closed_at IS NOT NULL THEN so.total_price ELSE 0 END), 0) as total_revenue,
                COALESCE(SUM(CASE WHEN so.closed_at IS NOT NULL THEN sop.unit_cost * sop.quantity ELSE 0 END), 0) as total_cost,
                COUNT(CASE WHEN so.status NOT IN ('Finalizada', 'Cancelada') THEN 1 END) as active_orders
            FROM service_orders so
            LEFT JOIN service_order_parts sop ON so.id = sop.service_order_id
            WHERE DATE(so.closed_at) = ?1 OR (DATE(so.created_at) <= ?1 AND so.status NOT IN ('Finalizada', 'Cancelada'))
        ";

        println!("[SEED] seed_financial_snapshots: Running query for date {}", snapshot_date);
        
        let (total_revenue, total_cost, active_orders): (f64, f64, i64) = conn
            .query_row(query, [&snapshot_date], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap_or((0.0, 0.0, 0));

        let net_profit = total_revenue - total_cost;
        
        println!("[SEED] seed_financial_snapshots: Query done, getting parts cost...");
        
        let parts_in_use_cost: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(sop.unit_cost * sop.quantity), 0)
                 FROM service_order_parts sop
                 JOIN service_orders so ON sop.service_order_id = so.id
                 WHERE so.status IN ('Em Manutenção', 'Aguardando Peça')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        println!("[SEED] seed_financial_snapshots: Inserting snapshot for {}", snapshot_date);
        
        conn.execute(
            "INSERT INTO financial_snapshots (id, snapshot_date, total_revenue, total_cost, net_profit, parts_in_use_cost, active_orders_count, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)",
            params![
                Uuid::new_v4().to_string(),
                snapshot_date,
                total_revenue,
                total_cost,
                net_profit,
                parts_in_use_cost,
                active_orders
            ],
        )
        .map_err(|e| e.to_string())?;
        
        println!("[SEED] seed_financial_snapshots: Inserted {}", snapshot_date);
    }

    println!("[SEED] seed_financial_snapshots: All done!");
    Ok(())
}

fn seed_checklist_templates(conn: &rusqlite::Connection) -> Result<(), String> {
    println!("[SEED] seed_checklist_templates: Starting...");
    
    let templates = vec![
        (
            "Checklist Smartphone",
            vec![
                "Tela/Touch",
                "Câmeras",
                "Microfone/Áudio",
                "Carregamento",
                "Botões",
                "Wi-Fi/Bluetooth",
                "Sensor de proximidade",
                "Alto-falante",
            ],
        ),
        (
            "Checklist Notebook",
            vec![
                "Teclado",
                "Tela/Monitor",
                "Webcam",
                "Portas USB",
                "Carregador",
                "Bateria",
                "Touchpad",
                "Ventilação",
            ],
        ),
        (
            "Checklist Tablet",
            vec![
                "Tela/Touch",
                "Bateria",
                "Câmeras",
                "Wi-Fi",
                "Bluetooth",
                "Alto-falante",
                "Botão Power",
                "Conector de carga",
            ],
        ),
        (
            "Checklist Impressora",
            vec![
                "Alimentação de papel",
                "Impressão",
                "Scanner",
                "Conexão USB",
                "Conexão Wi-Fi",
                "Roletes",
                "Cartucho/Toner",
                "Display/Led",
            ],
        ),
        (
            "Checklist Consoles",
            vec![
                "HDMI",
                "Controle",
                "Ventilação",
                "Fonte",
                "Disco",
                "USB",
                "Áudio",
                "Botões",
            ],
        ),
    ];

    for (title, items) in templates {
        let template_id = Uuid::new_v4().to_string();

        // Insert template
        conn.execute(
            "INSERT INTO checklist_templates (id, title, created_at)
             VALUES (?1, ?2, CURRENT_TIMESTAMP)",
            params![template_id, title],
        )
        .map_err(|e| e.to_string())?;

        // Insert template items
        for item in items {
            conn.execute(
                "INSERT INTO template_items (id, template_id, label)
                 VALUES (?1, ?2, ?3)",
                params![Uuid::new_v4().to_string(), template_id, item],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
