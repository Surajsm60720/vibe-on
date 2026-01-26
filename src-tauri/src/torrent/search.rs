use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub size: String,
    pub seeds: u32,
    pub leechers: u32,
    pub magnet: String,
    pub date: String,
    pub category: String,
    pub url: String,
}

pub async fn search_nyaa(
    query: String,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let sort = sort_by.unwrap_or_default(); // "seeders", "size", "id", "downloads"
    let order = sort_order.unwrap_or_else(|| "desc".to_string()); // "desc", "asc"

    let url = format!(
        "https://nyaa.si/?f=0&c=0_0&q={}&s={}&o={}",
        urlencoding::encode(&query),
        sort,
        order
    );
    println!("[Nyaa] Searching: {}", url);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch info: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Nyaa returned status: {}", res.status()));
    }

    let html = res
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let document = Html::parse_document(&html);

    // Nyaa table rows
    let row_selector = Selector::parse("tr.default, tr.success, tr.danger").unwrap();
    let link_selector = Selector::parse("a").unwrap();
    let magnet_selector = Selector::parse("a[href^='magnet:']").unwrap();

    let mut results = Vec::new();

    for row in document.select(&row_selector) {
        let tds: Vec<scraper::ElementRef> = row.select(&Selector::parse("td").unwrap()).collect();
        if tds.len() < 8 {
            continue;
        }

        // Category (td[0]) - skip for now or extract

        // Title (td[1])
        let title_node = tds[1];
        let title_links: Vec<scraper::ElementRef> = title_node.select(&link_selector).collect();

        let title_element = title_links
            .last()
            .ok_or("No title link")
            .map_err(|e: &str| e.to_string())?;

        let title = title_element.text().collect::<Vec<_>>().join(" ");
        let view_url = title_element.value().attr("href").unwrap_or_default();
        let full_url = format!("https://nyaa.si{}", view_url);

        // Magnet (td[2])
        let links_node = tds[2];
        let magnet = links_node
            .select(&magnet_selector)
            .next()
            .and_then(|el: scraper::ElementRef| el.value().attr("href"))
            .map(|s: &str| s.to_string())
            .unwrap_or_default();

        if magnet.is_empty() {
            continue;
        }

        // Size (td[3])
        let size = tds[3].text().collect::<Vec<_>>().join(" ");

        // Date (td[4])
        let date = tds[4].text().collect::<Vec<_>>().join(" ");

        // Seeds (td[5])
        let seeds = tds[5]
            .text()
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0);

        // Leechers (td[6])
        let leechers = tds[6]
            .text()
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0);

        results.push(SearchResult {
            title: title.trim().to_string(),
            size: size.trim().to_string(),
            seeds,
            leechers,
            magnet,
            date: date.trim().to_string(),
            category: "Anime".to_string(),
            url: full_url,
        });
    }

    Ok(results)
}
