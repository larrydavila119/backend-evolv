use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use crate::crud_inventory::get_product_by_id;
use crate::crud_sales::{ProductWithQuantity, SalesAsRecord};
use log::{info, warn, error};
use chrono::Local;
use rocket::State;

#[derive(Deserialize)]
pub struct PrintedSales {
    pub cashier: String,
    pub customer: Option<String>,
    pub payment_ref: String,
    pub products: Vec<ProductWithQuantity>,
    pub promocode: String,
    pub total_paid: f64,
    pub type_: String,
    pub currency: String,
    pub change: f64,
}

#[derive(Serialize)]
pub struct ReceiptItem {
    pub name: String,
    pub quantity: u8,
    pub price: f32,
    pub total: f32,
}

#[derive(Serialize)]
pub struct ReceiptJson {
    pub header: ReceiptHeader,
    pub payment_info: PaymentInfo,
    pub items: Vec<ReceiptItem>,
    pub totals: ReceiptTotals,
    pub footer: ReceiptFooter,
    pub last_sale_id: Option<String>,
}

#[derive(Serialize)]
pub struct ReceiptHeader {
    pub title: String,
    pub branch: String,
    pub date: String,
    pub cashier: String,
}

#[derive(Serialize)]
pub struct PaymentInfo {
    pub method: String,
    pub payment_ref: String,
    pub promocode: String,
}

#[derive(Serialize)]
pub struct ReceiptTotals {
    pub subtotal: f32,
    pub total: f32,
    pub currency: String,
}

#[derive(Serialize)]
pub struct ReceiptFooter {
    pub sale_id: Option<String>,
    pub qr_code_data: Option<String>,
}

pub async fn generate_receipt(
    sale: PrintedSales,
    database: &State<Surreal<Client>>,
) -> ReceiptJson {
    info!("Iniciando generación del JSON del recibo.");

    let mut items = Vec::new();
    let mut subtotal_price: f32 = 0.0;

    for product in &sale.products {
        let product_id = product.id.clone();
        let quantity = product.qnt as u8;

        match get_product_by_id(database, product_id.clone()).await {
            Ok(product_response) => {
                if let Some(product_name) = &product_response.name {
                    let price = product_response.price.unwrap_or(0.0);
                    let total = price as f32 * quantity as f32;
                    subtotal_price += total;
                    items.push(ReceiptItem {
                        name: product_name.clone(),
                        quantity,
                        price: price as f32,
                        total,
                    });
                } else {
                    warn!("El producto con ID {} no tiene nombre. Ignorando.", product_id);
                }
            }
            Err(e) => {
                error!("Error obteniendo el producto {}: {:?}", product_id, e);
            }
        }
    }

    let last_sale_id: Option<String> = get_last_sale_id(database).await;

    ReceiptJson {
        header: ReceiptHeader {
            title: "Choi Taekwondo".to_string(),
            branch: "Sucursal Reparto Serrano".to_string(),
            date: Local::now().format("%d-%m-%Y %H:%M").to_string(),
            cashier: sale.cashier.clone(),
        },
        payment_info: PaymentInfo {
            method: sale.type_.clone(),
            payment_ref: sale.payment_ref.clone(),
            promocode: sale.promocode.clone(),
        },
        items,
        totals: ReceiptTotals {
            subtotal: subtotal_price,
            total: sale.total_paid as f32,
            currency: sale.currency.clone(),
        },
        footer: ReceiptFooter {
            sale_id: last_sale_id.clone(),
            qr_code_data: last_sale_id.clone(),
        },
        last_sale_id,
    }
}

async fn get_last_sale_id(database: &State<Surreal<Client>>) -> Option<String> {
    let query = "SELECT id FROM (SELECT id, date FROM sales ORDER BY date DESC LIMIT 1);";
    match database.query(query).await {
        Ok(mut result) => {
            if let Some(record) = result
                .take::<Vec<SalesAsRecord>>(0)
                .ok()
                .and_then(|mut r| r.pop())
            {
                Some(record.id.to_string())
            } else {
                None
            }
        }
        Err(e) => {
            error!("Error al obtener el ID de la última venta: {:?}", e);
            None
        }
    }
}

