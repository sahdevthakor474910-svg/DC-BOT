use anyhow::Result;
use serde::Deserialize;
use tracing::{debug, warn};

use super::models::FreeGame;

// ── Epic Games public promotions API ────────────────────────────────────────

const EPIC_URL: &str =
    "https://store-site-backend-static.ak.epicgames.com/freeGamesPromotions\
     ?locale=en-US&country=US&allowCountries=US";

// ── Serde models ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct EpicResponse {
    data: EpicData,
}

#[derive(Debug, Deserialize)]
struct EpicData {
    #[serde(rename = "Catalog")]
    catalog: EpicCatalog,
}

#[derive(Debug, Deserialize)]
struct EpicCatalog {
    #[serde(rename = "searchStore")]
    search_store: EpicSearchStore,
}

#[derive(Debug, Deserialize)]
struct EpicSearchStore {
    elements: Vec<EpicElement>,
}

#[derive(Debug, Deserialize)]
struct EpicElement {
    id: String,
    title: String,
    description: Option<String>,
    #[serde(rename = "keyImages")]
    key_images: Vec<EpicImage>,
    price: Option<EpicPrice>,
    promotions: Option<EpicPromotions>,
    #[serde(rename = "catalogNs")]
    catalog_ns: Option<EpicCatalogNs>,
    #[serde(rename = "offerMappings")]
    offer_mappings: Option<Vec<EpicMapping>>,
}

#[derive(Debug, Deserialize)]
struct EpicImage {
    #[serde(rename = "type")]
    kind: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct EpicPrice {
    #[serde(rename = "totalPrice")]
    total_price: EpicTotalPrice,
}

#[derive(Debug, Deserialize)]
struct EpicTotalPrice {
    #[serde(rename = "discountPrice")]
    discount_price: i64,
    #[allow(dead_code)]
    #[serde(rename = "originalPrice")]
    original_price: i64,
    #[serde(rename = "fmtPrice")]
    fmt_price: Option<EpicFmtPrice>,
}

#[derive(Debug, Deserialize)]
struct EpicFmtPrice {
    #[serde(rename = "originalPrice")]
    original_price: String,
}

#[derive(Debug, Deserialize)]
struct EpicPromotions {
    #[serde(rename = "promotionalOffers")]
    promotional_offers: Vec<EpicOfferBlock>,
}

#[derive(Debug, Deserialize)]
struct EpicOfferBlock {
    #[serde(rename = "promotionalOffers")]
    offers: Vec<EpicOffer>,
}

#[derive(Debug, Deserialize)]
struct EpicOffer {
    #[allow(dead_code)]
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EpicCatalogNs {
    mappings: Option<Vec<EpicMapping>>,
}

#[derive(Debug, Deserialize, Clone)]
struct EpicMapping {
    #[serde(rename = "pageSlug")]
    page_slug: Option<String>,
}

// ── Public fetcher ───────────────────────────────────────────────────────────

pub async fn fetch_free_games(client: &reqwest::Client) -> Vec<FreeGame> {
    match try_fetch(client).await {
        Ok(games) => games,
        Err(e) => {
            warn!("Epic Games API error: {:#}", e);
            vec![]
        }
    }
}

async fn try_fetch(client: &reqwest::Client) -> Result<Vec<FreeGame>> {
    let resp: EpicResponse = client
        .get(EPIC_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut games = Vec::new();

    for el in resp.data.catalog.search_store.elements {
        // Must have active promotionalOffers at 100% discount
        let Some(promotions) = &el.promotions else { continue };
        if promotions.promotional_offers.is_empty() {
            continue;
        }

        // Find an active offer: must be currently promotional AND result in free (discountPrice==0)
        let is_free = el
            .price
            .as_ref()
            .map(|p| p.total_price.discount_price == 0)
            .unwrap_or(false);

        if !is_free {
            continue;
        }

        let active_offer = promotions
            .promotional_offers
            .iter()
            .flat_map(|b| b.offers.iter())
            .next(); // take the first active offer for the end date

        let Some(offer) = active_offer else { continue };

        // Parse end date
        let end_date = offer.end_date.as_deref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        // Prefer thumbnail images
        let thumbnail_url = el
            .key_images
            .iter()
            .find(|img| img.kind == "Thumbnail" || img.kind == "DieselGameBox")
            .map(|img| img.url.clone());

        // Build store URL from slug
        let slug = el
            .offer_mappings
            .as_ref()
            .and_then(|m| m.first())
            .and_then(|m| m.page_slug.clone())
            .or_else(|| {
                el.catalog_ns
                    .as_ref()
                    .and_then(|ns| ns.mappings.as_ref())
                    .and_then(|m| m.first())
                    .and_then(|m| m.page_slug.clone())
            })
            .unwrap_or_else(|| el.id.clone());

        let url = format!("https://store.epicgames.com/en-US/p/{}", slug);

        let original_price = el
            .price
            .as_ref()
            .and_then(|p| p.total_price.fmt_price.as_ref())
            .map(|fp| fp.original_price.clone())
            .filter(|s| s != "0" && s != "$0.00");

        debug!("Found Epic free game: {}", el.title);

        games.push(FreeGame {
            id: format!("epic::{}", el.id),
            title: el.title,
            description: el.description,
            original_price,
            store: "Epic Games".to_string(),
            url,
            thumbnail_url,
            end_date,
            claim_instructions: "Visit the Epic Games Store and click **Get** to claim!".to_string(),
        });
    }

    Ok(games)
}
