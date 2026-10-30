//! Network side of importing: fetching pages and photos.
//! Kept separate from the library so the database is never locked during a download.

use crate::parse::{is_pinterest, parse_page, pinterest_source_link, ParsedRecipe};
use crate::{Error, Result};
use std::time::Duration;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";

fn client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| Error::Network(e.to_string()))
}

/// Fetches a page and returns (final URL after redirects, HTML).
pub fn fetch_page(url: &str) -> Result<(String, String)> {
    let resp = client()?
        .get(url)
        .header("Accept", "text/html,application/xhtml+xml")
        .header("Accept-Language", "en,pt;q=0.8,es;q=0.6")
        .send()
        .map_err(|e| Error::Network(e.to_string()))?;
    let status = resp.status();
    let final_url = resp.url().to_string();
    if !status.is_success() {
        return Err(Error::Network(format!("the site answered {status}")));
    }
    let html = resp.text().map_err(|e| Error::Network(e.to_string()))?;
    Ok((final_url, html))
}

/// Fetches and parses a recipe URL, following a Pinterest pin to its source site.
pub fn fetch_recipe(url: &str) -> Result<ParsedRecipe> {
    let url = normalize_url(url)?;
    let (mut final_url, mut html) = fetch_page(&url)?;

    if is_pinterest(&final_url) {
        let source = pinterest_source_link(&html).ok_or(Error::PinWithoutLink)?;
        (final_url, html) = fetch_page(&source)?;
    }

    let parsed = parse_page(&html, &final_url);
    if !parsed.has_content() {
        return Err(Error::NotARecipe);
    }
    Ok(parsed)
}

/// Parses HTML the user saved or pasted; same rules as a fetched page.
pub fn parse_html(html: &str, url: &str) -> Result<ParsedRecipe> {
    let parsed = parse_page(html, url);
    if !parsed.has_content() {
        return Err(Error::NotARecipe);
    }
    Ok(parsed)
}

pub fn download_image(url: &str) -> Result<Vec<u8>> {
    let resp = client()?
        .get(url)
        .header("Accept", "image/avif,image/webp,image/*,*/*")
        .send()
        .map_err(|e| Error::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(Error::Network(format!("image answered {}", resp.status())));
    }
    let bytes = resp.bytes().map_err(|e| Error::Network(e.to_string()))?;
    Ok(bytes.to_vec())
}

fn normalize_url(input: &str) -> Result<String> {
    let t = input.trim();
    let with_scheme = if t.starts_with("http://") || t.starts_with("https://") { t.to_string() } else { format!("https://{t}") };
    url::Url::parse(&with_scheme).map(|u| u.to_string()).map_err(|_| Error::BadUrl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(normalize_url(" allrecipes.com/recipe/1 ").unwrap(), "https://allrecipes.com/recipe/1");
        assert!(normalize_url("not a url at all").is_err());
    }
}
