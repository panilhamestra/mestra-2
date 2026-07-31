use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize)] // Formato exato devolvido por https://fakestoreapi.com/products
pub struct RatingRaw {
    pub rate: f64,
    pub count: u32,
}

#[derive(Debug, Deserialize)]
pub struct ApiItemRaw {
    pub id: u32,
    pub title: String,
    pub price: f64,
    pub category: String,
    pub rating: RatingRaw,
}

#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct OrganizedItem {
    pub id: u32,
    pub name: String,
    pub category: String,
    pub price: f64,
    pub rating_count: u32,
    pub estimated_revenue: f64, // calculado em service.rs, não existe na API original
}
