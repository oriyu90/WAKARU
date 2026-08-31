//! Web link ingest (docs/04 §8). Fetch, refuse private targets (SSRF), turn HTML
//! into readable text, keep a copy in `derived/` so it reads offline (I-2).

use super::Unit;
use crate::error::{AppError, AppResult};
use std::net::IpAddr;
use std::path::Path;
use std::time::Duration;

pub struct WebResult {
    pub units: Vec<Unit>,
    pub title: Option<String>,
}

pub fn fetch_and_parse(url: &str, derived_dir: &Path, allow_private: bool) -> AppResult<WebResult> {
    let parsed = url::form_urlencoded_ok(url)?;
    guard_ssrf(&parsed, allow_private)?;

    let client = reqwest::blocking::Client::builder()
        .user_agent("WAKARU/0.2")
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| AppError::internal(format!("http client: {e}")))?;

    let resp = client
        .get(url)
        .send()
        .map_err(|e| AppError::new("WEB_FETCH", "error.web.fetch", e.to_string()).retriable())?;

    if !resp.status().is_success() {
        return Err(AppError::new(
            "WEB_FETCH",
            "error.web.fetch",
            format!("HTTP {}", resp.status()),
        ));
    }
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if !ctype.contains("text/html") && !ctype.contains("text/plain") && !ctype.is_empty() {
        return Err(AppError::new(
            "SOURCE_UNSUPPORTED_FORMAT",
            "error.source.unsupported",
            format!("web content type {ctype} — download the file and add it directly"),
        ));
    }

    let html = resp
        .text()
        .map_err(|e| AppError::new("WEB_FETCH", "error.web.fetch", e.to_string()))?;
    let (title, sections) = extract_readable(&html);

    std::fs::create_dir_all(derived_dir)?;
    let md = sections
        .iter()
        .map(|(h, b)| match h {
            Some(h) => format!("## {h}\n\n{b}"),
            None => b.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    std::fs::write(derived_dir.join("reader.md"), &md)?;
    std::fs::write(derived_dir.join("original.html"), &html)?;

    let mut units = Vec::new();
    for (i, (h, b)) in sections.into_iter().enumerate() {
        if b.trim().is_empty() {
            continue;
        }
        units.push(Unit {
            ordinal: (i + 1) as u32,
            kind: "section",
            title: h.clone(),
            text: match h {
                Some(h) => format!("# {h}\n\n{b}"),
                None => b,
            },
            locator: serde_json::json!({ "t": "anchor", "selector": format!("#sec-{}", i + 1) }),
        });
    }
    if units.is_empty() {
        return Err(AppError::new(
            "SOURCE_EMPTY",
            "error.source.empty",
            "no readable content",
        ));
    }

    Ok(WebResult { units, title })
}

fn guard_ssrf(url: &UrlParts, allow_private: bool) -> AppResult<()> {
    if url.scheme == "file" || url.scheme == "data" {
        return Err(deny("file/data URLs are not allowed"));
    }
    if url.scheme != "http" && url.scheme != "https" {
        return Err(deny("only http(s) URLs are supported"));
    }
    let host = url.host.to_ascii_lowercase();
    if allow_private {
        return Ok(());
    }
    if host == "localhost" || host.ends_with(".localhost") || host == "0.0.0.0" {
        return Err(deny("localhost is blocked (enable in settings to allow)"));
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private(&ip) {
            return Err(deny("private / loopback addresses are blocked"));
        }
    }
    Ok(())
}

fn is_private(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.octets()[0] == 0
                || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
            // CGNAT
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified() || v6.segments()[0] & 0xfe00 == 0xfc00
        }
    }
}

fn deny(msg: &str) -> AppError {
    AppError::new("WEB_BLOCKED", "error.web.blocked", msg)
}

struct UrlParts {
    scheme: String,
    host: String,
}

mod url {
    use super::{AppError, AppResult, UrlParts};

    /// Minimal URL split — scheme + host — enough for the SSRF guard. reqwest
    /// itself does the real parsing on the request.
    pub fn form_urlencoded_ok(raw: &str) -> AppResult<UrlParts> {
        let raw = raw.trim();
        let (scheme, rest) = raw
            .split_once("://")
            .ok_or_else(|| AppError::new("WEB_BLOCKED", "error.web.blocked", "not a URL"))?;
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        let after_auth = authority.rsplit('@').next().unwrap_or(authority);
        let host = if let Some(rest) = after_auth.strip_prefix('[') {
            // bracketed IPv6 literal
            rest.split(']').next().unwrap_or(rest)
        } else {
            after_auth.split(':').next().unwrap_or(after_auth)
        };
        if host.is_empty() {
            return Err(AppError::new("WEB_BLOCKED", "error.web.blocked", "no host"));
        }
        Ok(UrlParts {
            scheme: scheme.to_ascii_lowercase(),
            host: host.to_string(),
        })
    }
}

/// Very small readability pass: drop script/style/nav/header/footer/aside, then
/// take headings and paragraph text in document order.
fn extract_readable(html: &str) -> (Option<String>, Vec<(Option<String>, String)>) {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);

    let title = doc
        .select(&Selector::parse("title").unwrap())
        .next()
        .map(|t| t.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty());

    let body_sel = Selector::parse("body").unwrap();
    let Some(body) = doc.select(&body_sel).next() else {
        return (title, vec![]);
    };

    let block_sel = Selector::parse("h1, h2, h3, p, li, blockquote, pre").unwrap();
    let skip_ancestors = [
        "script", "style", "nav", "header", "footer", "aside", "noscript",
    ];

    let mut sections: Vec<(Option<String>, String)> = vec![(None, String::new())];
    for el in body.select(&block_sel) {
        if el
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .any(|a| skip_ancestors.contains(&a.value().name()))
        {
            continue;
        }
        let text = el
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if text.is_empty() {
            continue;
        }
        match el.value().name() {
            "h1" | "h2" | "h3" => sections.push((Some(text), String::new())),
            _ => {
                let last = sections.last_mut().unwrap();
                last.1.push_str(&text);
                last.1.push_str("\n\n");
            }
        }
    }
    (title, sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(u: &str) -> UrlParts {
        url::form_urlencoded_ok(u).unwrap()
    }

    #[test]
    fn ac_1_9_private_and_local_targets_are_blocked() {
        for bad in [
            "http://localhost/x",
            "http://127.0.0.1/",
            "https://10.0.0.5/a",
            "http://192.168.1.1",
            "http://172.16.9.9/",
            "http://169.254.1.1/",
            "http://[::1]/",
            "file:///etc/passwd",
        ] {
            let p = url::form_urlencoded_ok(bad).unwrap_or(UrlParts {
                scheme: "file".into(),
                host: "x".into(),
            });
            assert!(guard_ssrf(&p, false).is_err(), "should block {bad}");
        }
    }

    #[test]
    fn public_host_passes_the_guard() {
        assert!(guard_ssrf(&parts("https://example.com/page"), false).is_ok());
    }

    #[test]
    fn allow_private_opt_in_lets_localhost_through() {
        assert!(guard_ssrf(&parts("http://localhost:8080/"), true).is_ok());
    }

    #[test]
    fn readable_extraction_pulls_headings_and_paragraphs() {
        let html = r#"<html><head><title>T</title></head><body>
            <nav>skip me</nav><h1>Intro</h1><p>Hello world.</p>
            <script>var x=1;</script><h2>More</h2><p>Second para.</p></body></html>"#;
        let (title, secs) = extract_readable(html);
        assert_eq!(title.as_deref(), Some("T"));
        let joined: String = secs.iter().map(|(_, b)| b.clone()).collect();
        assert!(joined.contains("Hello world."));
        assert!(joined.contains("Second para."));
        assert!(!joined.contains("skip me"));
        assert!(!joined.contains("var x"));
    }
}
