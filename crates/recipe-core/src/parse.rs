//! Turns a recipe web page into a [`ParsedRecipe`].
//!
//! Order of attempts: schema.org JSON-LD, then microdata (`itemprop`), then a
//! heuristic that looks for "Ingredients" / "Method" headings followed by lists.
//! Whatever is found becomes an editable draft; nothing is invented.

use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Section {
    pub title: Option<String>,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParseMethod {
    JsonLd,
    Microdata,
    Heuristic,
    #[default]
    Nothing,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ParsedRecipe {
    pub title: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub site_name: Option<String>,
    pub source_url: Option<String>,
    pub servings: Option<String>,
    pub prep_min: Option<i64>,
    pub cook_min: Option<i64>,
    pub total_min: Option<i64>,
    pub ingredients: Vec<Section>,
    pub steps: Vec<Section>,
    pub images: Vec<String>,
    pub json_ld: Option<String>,
    pub method: ParseMethod,
}

impl ParsedRecipe {
    pub fn has_content(&self) -> bool {
        self.ingredients.iter().any(|s| !s.items.is_empty()) || self.steps.iter().any(|s| !s.items.is_empty())
    }
}

/// Parses a page. `page_url` is used to resolve relative image links and as the source URL.
pub fn parse_page(html: &str, page_url: &str) -> ParsedRecipe {
    let doc = Html::parse_document(html);
    let base = Url::parse(page_url).ok();

    let mut r = from_json_ld(&doc)
        .or_else(|| from_microdata(&doc))
        .or_else(|| from_heuristic(&doc))
        .unwrap_or_default();

    // Fill gaps from page metadata.
    if r.title.trim().is_empty() {
        r.title = meta(&doc, "og:title")
            .or_else(|| first_text(&doc, "h1"))
            .or_else(|| first_text(&doc, "title"))
            .unwrap_or_else(|| "Untitled recipe".to_string());
    }
    if r.description.is_none() {
        r.description = meta(&doc, "og:description").or_else(|| meta_name(&doc, "description"));
    }
    if r.images.is_empty() {
        if let Some(img) = meta(&doc, "og:image") {
            r.images.push(img);
        }
    }
    r.site_name = r
        .site_name
        .or_else(|| meta(&doc, "og:site_name"))
        .or_else(|| base.as_ref().and_then(|u| u.host_str()).map(|h| h.trim_start_matches("www.").to_string()));
    r.source_url = Some(page_url.to_string());

    // Absolute, de-duplicated image URLs.
    let mut seen = Vec::new();
    for img in r.images.drain(..) {
        let abs = match &base {
            Some(b) => b.join(&img).map(|u| u.to_string()).unwrap_or(img),
            None => img,
        };
        if abs.starts_with("http") && !seen.contains(&abs) {
            seen.push(abs);
        }
    }
    r.images = seen;
    r.title = clean(&r.title);
    r
}

// ---------- JSON-LD ----------

fn from_json_ld(doc: &Html) -> Option<ParsedRecipe> {
    let sel = Selector::parse(r#"script[type="application/ld+json"]"#).unwrap();
    for script in doc.select(&sel) {
        let raw: String = script.text().collect();
        let Some(value) = parse_json_lenient(&raw) else { continue };
        if let Some(recipe) = find_recipe(&value) {
            let mut r = recipe_from_value(recipe);
            r.json_ld = serde_json::to_string(recipe).ok();
            r.method = ParseMethod::JsonLd;
            return Some(r);
        }
    }
    None
}

fn parse_json_lenient(raw: &str) -> Option<Value> {
    let t = raw.trim().trim_end_matches(';');
    serde_json::from_str(t).ok().or_else(|| {
        // Some sites put raw newlines/tabs inside strings.
        let fixed: String = t.chars().map(|c| if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c }).collect();
        serde_json::from_str(&fixed).ok()
    })
}

fn is_type(v: &Value, t: &str) -> bool {
    match v.get("@type") {
        Some(Value::String(s)) => s.eq_ignore_ascii_case(t) || s.ends_with(&format!("/{t}")),
        Some(Value::Array(a)) => a.iter().any(|x| x.as_str().map_or(false, |s| s.eq_ignore_ascii_case(t))),
        _ => false,
    }
}

fn find_recipe(v: &Value) -> Option<&Value> {
    match v {
        Value::Object(map) => {
            if is_type(v, "Recipe") {
                return Some(v);
            }
            for key in ["@graph", "mainEntity", "mainEntityOfPage", "itemListElement", "item"] {
                if let Some(found) = map.get(key).and_then(find_recipe) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(find_recipe),
        _ => None,
    }
}

fn recipe_from_value(v: &Value) -> ParsedRecipe {
    let ingredients: Vec<String> = string_list(v.get("recipeIngredient").or_else(|| v.get("ingredients")))
        .into_iter()
        .map(|s| clean(&s))
        .filter(|s| !s.is_empty())
        .collect();

    ParsedRecipe {
        title: v.get("name").and_then(text_of).unwrap_or_default(),
        description: v.get("description").and_then(text_of).filter(|s| !s.is_empty()),
        author: v.get("author").and_then(name_of),
        site_name: v.get("publisher").and_then(name_of),
        servings: v.get("recipeYield").and_then(yield_of),
        prep_min: v.get("prepTime").and_then(Value::as_str).and_then(parse_duration),
        cook_min: v.get("cookTime").and_then(Value::as_str).and_then(parse_duration),
        total_min: v.get("totalTime").and_then(Value::as_str).and_then(parse_duration),
        ingredients: if ingredients.is_empty() { vec![] } else { vec![Section { title: None, items: ingredients }] },
        steps: instructions_of(v.get("recipeInstructions")),
        images: images_of(v.get("image")),
        ..Default::default()
    }
}

fn text_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(clean(s)),
        Value::Number(n) => Some(n.to_string()),
        Value::Array(a) => a.iter().find_map(text_of),
        Value::Object(_) => v.get("text").or_else(|| v.get("name")).and_then(text_of),
        _ => None,
    }
}

fn name_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(clean(s)),
        Value::Object(_) => v.get("name").and_then(name_of),
        Value::Array(a) => {
            let names: Vec<String> = a.iter().filter_map(name_of).collect();
            if names.is_empty() { None } else { Some(names.join(", ")) }
        }
        _ => None,
    }
    .filter(|s| !s.is_empty())
}

fn yield_of(v: &Value) -> Option<String> {
    match v {
        Value::Array(a) => {
            // Prefer the most descriptive entry ("4 servings" over "4").
            a.iter().filter_map(yield_of).max_by_key(|s| s.len())
        }
        other => text_of(other),
    }
    .filter(|s| !s.is_empty())
}

fn string_list(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Array(a)) => a.iter().filter_map(text_of).collect(),
        Some(Value::String(s)) => s.lines().map(str::to_string).collect(),
        _ => vec![],
    }
}

fn instructions_of(v: Option<&Value>) -> Vec<Section> {
    let mut sections: Vec<Section> = vec![];
    let mut loose = Section::default();
    collect_steps(v, &mut loose, &mut sections);
    if !loose.items.is_empty() {
        sections.insert(0, loose);
    }
    for s in &mut sections {
        s.items = s.items.iter().map(|t| clean(t)).filter(|t| !t.is_empty()).collect();
    }
    sections.retain(|s| !s.items.is_empty());
    sections
}

fn collect_steps(v: Option<&Value>, current: &mut Section, sections: &mut Vec<Section>) {
    match v {
        Some(Value::String(s)) => {
            // One block of text: split into paragraphs / lines / numbered steps.
            let text = html_to_text(s);
            for line in text.split('\n').map(str::trim).filter(|l| !l.is_empty()) {
                current.items.push(strip_step_number(line));
            }
        }
        Some(Value::Array(a)) => {
            for item in a {
                collect_steps(Some(item), current, sections);
            }
        }
        Some(obj @ Value::Object(_)) => {
            if is_type(obj, "HowToSection") || (obj.get("itemListElement").is_some() && !is_type(obj, "HowToStep")) {
                let mut sec = Section { title: obj.get("name").and_then(text_of).filter(|s| !s.is_empty()), items: vec![] };
                let mut nested = vec![];
                collect_steps(obj.get("itemListElement"), &mut sec, &mut nested);
                sections.push(sec);
                sections.extend(nested);
            } else if let Some(t) = obj.get("text").or_else(|| obj.get("name")).and_then(text_of) {
                current.items.push(t);
            }
        }
        _ => {}
    }
}

fn images_of(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a.iter().flat_map(|x| images_of(Some(x))).collect(),
        Some(o @ Value::Object(_)) => o
            .get("url")
            .or_else(|| o.get("contentUrl"))
            .and_then(Value::as_str)
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        _ => vec![],
    }
}

// ---------- Microdata ----------

fn from_microdata(doc: &Html) -> Option<ParsedRecipe> {
    let root_sel = Selector::parse(r#"[itemtype*="schema.org/Recipe"]"#).unwrap();
    let root = doc.select(&root_sel).next()?;
    let pick = |prop: &str| Selector::parse(&format!(r#"[itemprop="{prop}"]"#)).unwrap();

    let ingredients: Vec<String> = root
        .select(&pick("recipeIngredient"))
        .chain(root.select(&pick("ingredients")))
        .map(element_text)
        .filter(|s| !s.is_empty())
        .collect();

    let li = Selector::parse("li").unwrap();
    let mut steps = vec![];
    for el in root.select(&pick("recipeInstructions")) {
        let items: Vec<String> = el.select(&li).map(element_text).filter(|s| !s.is_empty()).collect();
        if items.is_empty() {
            let t = element_text(el);
            if !t.is_empty() {
                steps.push(t);
            }
        } else {
            steps.extend(items);
        }
    }
    if ingredients.is_empty() && steps.is_empty() {
        return None;
    }
    let attr_or_text = |prop: &str| {
        root.select(&pick(prop)).next().map(|e| {
            e.value().attr("content").map(clean).unwrap_or_else(|| element_text(e))
        })
    };
    let image = root
        .select(&pick("image"))
        .next()
        .and_then(|e| e.value().attr("src").or(e.value().attr("content")).or(e.value().attr("href")).map(str::to_string));

    Some(ParsedRecipe {
        title: attr_or_text("name").unwrap_or_default(),
        description: attr_or_text("description").filter(|s| !s.is_empty()),
        author: attr_or_text("author").filter(|s| !s.is_empty()),
        servings: attr_or_text("recipeYield").filter(|s| !s.is_empty()),
        prep_min: root.select(&pick("prepTime")).next().and_then(duration_attr),
        cook_min: root.select(&pick("cookTime")).next().and_then(duration_attr),
        total_min: root.select(&pick("totalTime")).next().and_then(duration_attr),
        ingredients: if ingredients.is_empty() { vec![] } else { vec![Section { title: None, items: ingredients }] },
        steps: if steps.is_empty() { vec![] } else { vec![Section { title: None, items: steps }] },
        images: image.into_iter().collect(),
        method: ParseMethod::Microdata,
        ..Default::default()
    })
}

fn duration_attr(e: ElementRef) -> Option<i64> {
    e.value().attr("content").or(e.value().attr("datetime")).and_then(parse_duration)
}

// ---------- Heuristic ----------

const INGREDIENT_WORDS: &[&str] = &["ingredient", "ingrediente", "you will need", "what you need"];
const STEP_WORDS: &[&str] = &[
    "instruction", "method", "direction", "preparation", "steps", "how to make",
    "modo de preparo", "modo de fazer", "preparo", "preparación", "instrucciones", "elaboración",
];

fn from_heuristic(doc: &Html) -> Option<ParsedRecipe> {
    let headings = Selector::parse("h1, h2, h3, h4, h5, strong, b, p").unwrap();
    let mut ingredients = vec![];
    let mut steps = vec![];
    for h in doc.select(&headings) {
        let label = element_text(h).to_lowercase();
        if label.is_empty() || label.len() > 40 {
            continue;
        }
        if ingredients.is_empty() && INGREDIENT_WORDS.iter().any(|w| label.contains(w)) {
            ingredients = list_after(h);
        } else if steps.is_empty() && STEP_WORDS.iter().any(|w| label.contains(w)) {
            steps = list_after(h);
        }
    }
    if ingredients.is_empty() && steps.is_empty() {
        return None;
    }
    Some(ParsedRecipe {
        ingredients: if ingredients.is_empty() { vec![] } else { vec![Section { title: None, items: ingredients }] },
        steps: if steps.is_empty() { vec![] } else { vec![Section { title: None, items: steps }] },
        method: ParseMethod::Heuristic,
        ..Default::default()
    })
}

/// The items of the first list that follows `start` (checking its following
/// siblings, then those of its parent, so `<div><h3>..</h3></div><ul>` works).
fn list_after(start: ElementRef) -> Vec<String> {
    let li = Selector::parse("li").unwrap();
    let mut node = Some(start);
    for _ in 0..3 {
        let Some(n) = node else { break };
        for sib in n.next_siblings().filter_map(ElementRef::wrap).take(6) {
            let name = sib.value().name();
            if matches!(name, "h1" | "h2" | "h3" | "h4") {
                break;
            }
            let items: Vec<String> = if matches!(name, "ul" | "ol") {
                sib.select(&li).map(element_text).collect()
            } else {
                sib.select(&li).map(element_text).collect()
            };
            let items: Vec<String> = items.into_iter().filter(|s| !s.is_empty()).collect();
            if !items.is_empty() {
                return items;
            }
        }
        node = n.parent().and_then(ElementRef::wrap);
    }
    vec![]
}

// ---------- Pinterest ----------

pub fn is_pinterest(url: &str) -> bool {
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.contains("pinterest.") || h == "pin.it"))
        .unwrap_or(false)
}

/// Finds the site a pin links out to.
pub fn pinterest_source_link(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    if let Some(see_also) = meta(&doc, "og:see_also") {
        if !is_pinterest(&see_also) {
            return Some(see_also);
        }
    }
    for key in ["\"link\":\"", "\"link\": \"", "\"url\":\"", "\"sourceUrl\":\""] {
        let mut rest = html;
        while let Some(pos) = rest.find(key) {
            rest = &rest[pos + key.len()..];
            let Some(end) = rest.find('"') else { break };
            let candidate = rest[..end].replace("\\/", "/").replace("\\u002F", "/").replace("\\u0026", "&");
            if candidate.starts_with("http") && !is_pinterest(&candidate) && !candidate.contains("pinimg.com") {
                return Some(candidate);
            }
        }
    }
    None
}

// ---------- Helpers ----------

/// ISO 8601 duration ("PT1H15M", "P0DT0H20M") to whole minutes.
pub fn parse_duration(s: &str) -> Option<i64> {
    let s = s.trim().to_ascii_uppercase();
    let rest = s.strip_prefix('P')?;
    let (mut minutes, mut num, mut in_time) = (0f64, String::new(), false);
    for c in rest.chars() {
        match c {
            'T' => in_time = true,
            '0'..='9' | '.' | ',' => num.push(if c == ',' { '.' } else { c }),
            _ => {
                let n: f64 = num.parse().ok()?;
                num.clear();
                minutes += match (c, in_time) {
                    ('D', _) => n * 1440.0,
                    ('W', false) => n * 10080.0,
                    ('H', true) => n * 60.0,
                    ('M', true) => n,
                    ('S', true) => n / 60.0,
                    _ => 0.0,
                };
            }
        }
    }
    let m = minutes.round() as i64;
    (m > 0).then_some(m)
}

fn strip_step_number(line: &str) -> String {
    let t = line.trim_start();
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && digits <= 2 {
        let after = &t[digits..];
        if let Some(r) = after.strip_prefix('.').or_else(|| after.strip_prefix(')')) {
            return r.trim().to_string();
        }
    }
    t.to_string()
}

fn html_to_text(s: &str) -> String {
    if !s.contains('<') && !s.contains('&') {
        return s.to_string();
    }
    let with_breaks = s
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "</p>\n")
        .replace("</li>", "</li>\n");
    let frag = Html::parse_fragment(&with_breaks);
    frag.root_element().text().collect()
}

/// Strips tags, decodes entities and collapses whitespace.
pub fn clean(s: &str) -> String {
    let text = if s.contains('<') || s.contains('&') {
        Html::parse_fragment(s).root_element().text().collect::<String>()
    } else {
        s.to_string()
    };
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn element_text(e: ElementRef) -> String {
    clean(&e.text().collect::<String>())
}

fn first_text(doc: &Html, css: &str) -> Option<String> {
    let sel = Selector::parse(css).ok()?;
    doc.select(&sel).next().map(element_text).filter(|s| !s.is_empty())
}

fn meta(doc: &Html, property: &str) -> Option<String> {
    let sel = Selector::parse(&format!(r#"meta[property="{property}"]"#)).ok()?;
    doc.select(&sel).next()?.value().attr("content").map(clean).filter(|s| !s.is_empty())
}

fn meta_name(doc: &Html, name: &str) -> Option<String> {
    let sel = Selector::parse(&format!(r#"meta[name="{name}"]"#)).ok()?;
    doc.select(&sel).next()?.value().attr("content").map(clean).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("PT1H15M"), Some(75));
        assert_eq!(parse_duration("PT20M"), Some(20));
        assert_eq!(parse_duration("P0DT0H45M"), Some(45));
        assert_eq!(parse_duration("PT90S"), Some(2));
        assert_eq!(parse_duration("PT0M"), None);
        assert_eq!(parse_duration("20 minutes"), None);
    }

    #[test]
    fn json_ld_in_graph_with_sections() {
        let html = r#"<html><head>
        <meta property="og:site_name" content="Test Kitchen">
        <script type="application/ld+json">
        {"@context":"https://schema.org","@graph":[
          {"@type":"WebPage","name":"Page"},
          {"@type":["Recipe"],"name":"Caldo Verde &amp; Chouriço",
           "author":[{"@type":"Person","name":"Ana"}],
           "recipeYield":["4","4 servings"],
           "prepTime":"PT15M","cookTime":"PT30M","totalTime":"PT45M",
           "image":[{"@type":"ImageObject","url":"/img/caldo.jpg"}],
           "recipeIngredient":["1 lb potatoes","4 cups <b>kale</b>"],
           "recipeInstructions":[
             {"@type":"HowToSection","name":"Soup","itemListElement":[
               {"@type":"HowToStep","text":"Boil the potatoes."},
               {"@type":"HowToStep","text":"Add the kale."}]},
             {"@type":"HowToSection","name":"To serve","itemListElement":[
               {"@type":"HowToStep","text":"Drizzle with olive oil."}]}
           ]}
        ]}
        </script></head><body></body></html>"#;
        let r = parse_page(html, "https://example.com/recipes/caldo");
        assert_eq!(r.method, ParseMethod::JsonLd);
        assert_eq!(r.title, "Caldo Verde & Chouriço");
        assert_eq!(r.author.as_deref(), Some("Ana"));
        assert_eq!(r.servings.as_deref(), Some("4 servings"));
        assert_eq!((r.prep_min, r.cook_min, r.total_min), (Some(15), Some(30), Some(45)));
        assert_eq!(r.ingredients[0].items, vec!["1 lb potatoes", "4 cups kale"]);
        assert_eq!(r.steps.len(), 2);
        assert_eq!(r.steps[0].title.as_deref(), Some("Soup"));
        assert_eq!(r.steps[0].items, vec!["Boil the potatoes.", "Add the kale."]);
        assert_eq!(r.steps[1].items, vec!["Drizzle with olive oil."]);
        assert_eq!(r.images, vec!["https://example.com/img/caldo.jpg"]);
        assert_eq!(r.site_name.as_deref(), Some("Test Kitchen"));
    }

    #[test]
    fn json_ld_plain_strings() {
        let html = r#"<script type="application/ld+json">
        {"@type":"Recipe","name":"Toast","image":"https://cdn.x/t.jpg",
         "recipeIngredient":["2 slices bread"],
         "recipeInstructions":"1. Toast the bread.<br>2. Butter it."}
        </script>"#;
        let r = parse_page(html, "https://x.com/toast");
        assert_eq!(r.steps[0].items, vec!["Toast the bread.", "Butter it."]);
        assert_eq!(r.images, vec!["https://cdn.x/t.jpg"]);
    }

    #[test]
    fn microdata() {
        let html = r#"<div itemscope itemtype="http://schema.org/Recipe">
          <h1 itemprop="name">Pancakes</h1>
          <meta itemprop="totalTime" content="PT25M">
          <ul><li itemprop="recipeIngredient">1 cup flour</li><li itemprop="recipeIngredient">1 egg</li></ul>
          <ol itemprop="recipeInstructions"><li>Mix.</li><li>Fry.</li></ol>
        </div>"#;
        let r = parse_page(html, "https://p.com/a");
        assert_eq!(r.method, ParseMethod::Microdata);
        assert_eq!(r.title, "Pancakes");
        assert_eq!(r.total_min, Some(25));
        assert_eq!(r.ingredients[0].items, vec!["1 cup flour", "1 egg"]);
        assert_eq!(r.steps[0].items, vec!["Mix.", "Fry."]);
    }

    #[test]
    fn heuristic_portuguese() {
        let html = r#"<html><head><meta property="og:title" content="Bolo de Cenoura">
          <meta property="og:image" content="https://b.br/bolo.jpg"></head><body>
          <h2>Ingredientes</h2><ul><li>3 cenouras</li><li>4 ovos</li></ul>
          <h2>Modo de preparo</h2><ol><li>Bata tudo.</li><li>Asse por 40 minutos.</li></ol>
        </body></html>"#;
        let r = parse_page(html, "https://b.br/bolo");
        assert_eq!(r.method, ParseMethod::Heuristic);
        assert_eq!(r.title, "Bolo de Cenoura");
        assert_eq!(r.ingredients[0].items, vec!["3 cenouras", "4 ovos"]);
        assert_eq!(r.steps[0].items, vec!["Bata tudo.", "Asse por 40 minutos."]);
        assert_eq!(r.images, vec!["https://b.br/bolo.jpg"]);
        assert_eq!(r.site_name.as_deref(), Some("b.br"));
    }

    #[test]
    fn nothing_found_still_gives_title() {
        let r = parse_page("<html><head><title>Just a blog</title></head><body><p>hi</p></body></html>", "https://x.com/");
        assert_eq!(r.title, "Just a blog");
        assert!(!r.has_content());
    }

    #[test]
    fn pinterest_link() {
        assert!(is_pinterest("https://www.pinterest.ca/pin/12345/"));
        assert!(is_pinterest("https://pin.it/abc"));
        assert!(!is_pinterest("https://www.allrecipes.com/x"));
        let html = r#"<script>{"images":{"url":"https:\/\/i.pinimg.com\/a.jpg"},"link":"https:\/\/www.budgetbytes.com\/soup\/"}</script>"#;
        assert_eq!(pinterest_source_link(html).as_deref(), Some("https://www.budgetbytes.com/soup/"));
    }
}
