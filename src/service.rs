use crate::models::{ApiItemRaw, OrganizedItem};
use crate::state::AppState;

// Chegada
async fn fetch_raw_items(state: &AppState) -> Result<Vec<ApiItemRaw>, reqwest::Error> {
    state
        .http_client
        .get(&state.api_url)
        .send()
        .await?
        .json::<Vec<ApiItemRaw>>()
        .await
}

// Operação
fn calculate_estimated_revenue(price: f64, rating_count: u32) -> f64 {
    price * rating_count as f64
}

// Partida
fn organize_item(item: ApiItemRaw, estimated_revenue: f64) -> OrganizedItem {
    OrganizedItem {
        id: item.id,
        name: item.title,
        category: item.category,
        price: item.price,
        rating_count: item.rating.count,
        estimated_revenue,
    }
}

// Ciclo completo: Chegada -> Operação -> Partida
pub async fn fetch_and_organize(state: &AppState) -> Result<Vec<OrganizedItem>, reqwest::Error> {
    let raw_items = fetch_raw_items(state).await?;

    let organized_items = raw_items
        .into_iter()
        .map(|item| {
            let estimated_revenue = calculate_estimated_revenue(item.price, item.rating.count);
            organize_item(item, estimated_revenue)
        })
        .collect();

    Ok(organized_items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_revenue_by_multiplying_price_and_rating_count() {
        let result = calculate_estimated_revenue(10.0, 5);
        assert_eq!(result, 50.0);
    }
}
