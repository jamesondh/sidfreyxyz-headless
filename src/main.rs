use once_cell::sync::Lazy;
use std::io::Cursor;
use std::net::ToSocketAddrs;
use tiny_http::{Header, Method, Response, Server, StatusCode};
use url::form_urlencoded;
use chrono::Local;

#[derive(Clone, Copy)]
enum Engine {
    Google,
    YouTube,
    Wikipedia,
    Claude,
    ChatGPT,
    GoogleImages,
    Wolfram,
    Reddit,
    Bing,
    Amazon,
    Twitter,
    GitHub,
    EBay,
    DuckDuckGo,
    Perplexity,
}

impl Engine {
    fn to_string(&self) -> &'static str {
        match self {
            Engine::Google => "Google",
            Engine::YouTube => "YouTube",
            Engine::Wikipedia => "Wikipedia",
            Engine::Claude => "Claude",
            Engine::ChatGPT => "ChatGPT",
            Engine::GoogleImages => "Google Images",
            Engine::Wolfram => "Wolfram Alpha",
            Engine::Reddit => "Reddit",
            Engine::Bing => "Bing",
            Engine::Amazon => "Amazon",
            Engine::Twitter => "Twitter",
            Engine::GitHub => "GitHub",
            Engine::EBay => "eBay",
            Engine::DuckDuckGo => "DuckDuckGo",
            Engine::Perplexity => "Perplexity",
        }
    }
}

static DEFAULT_ENGINE: Engine = Engine::Perplexity;

static BANG_MAP: Lazy<Vec<(&'static str, Engine)>> = Lazy::new(|| {
    vec![
        ("!g", Engine::Google), ("!google", Engine::Google),
        ("!yt", Engine::YouTube), ("!youtube", Engine::YouTube),
        ("!w", Engine::Wikipedia), ("!wiki", Engine::Wikipedia), ("!wikipedia", Engine::Wikipedia),
        ("!cl", Engine::Claude), ("!claude", Engine::Claude),
        ("!gpt", Engine::ChatGPT), ("!chat", Engine::ChatGPT), ("!chatgpt", Engine::ChatGPT),
        ("!gi", Engine::GoogleImages), ("!img", Engine::GoogleImages), ("!image", Engine::GoogleImages),
        ("!wa", Engine::Wolfram), ("!wolfram", Engine::Wolfram), ("!wolframalpha", Engine::Wolfram),
        ("!r", Engine::Reddit), ("!reddit", Engine::Reddit),
        ("!b", Engine::Bing), ("!bing", Engine::Bing),
        ("!a", Engine::Amazon), ("!amazon", Engine::Amazon),
        ("!x", Engine::Twitter), ("!tw", Engine::Twitter), ("!twitter", Engine::Twitter),
        ("!github", Engine::GitHub), ("!gh", Engine::GitHub),
        ("!ebay", Engine::EBay),
        ("!ddg", Engine::DuckDuckGo), ("!duckduckgo", Engine::DuckDuckGo),
        ("!p", Engine::Perplexity), ("!perplexity", Engine::Perplexity),
    ]
});

fn engine_url(engine: Engine, q: &str) -> String {
    let e = urlencoding::encode(q);
    match engine {
        Engine::Google => format!("https://www.google.com/search?q={e}"),
        Engine::YouTube => format!("https://www.youtube.com/results?search_query={e}"),
        Engine::Wikipedia => format!("https://en.wikipedia.org/wiki/Special:Search?search={e}"),
        Engine::Claude => format!("https://claude.ai/new?q={e}"),
        Engine::ChatGPT => format!("https://chat.openai.com/?q={e}"),
        Engine::GoogleImages => format!("https://www.google.com/search?q={e}&tbm=isch"),
        Engine::Wolfram => format!("https://www.wolframalpha.com/input/?i={e}"),
        Engine::Reddit => format!("https://www.reddit.com/search?q={e}"),
        Engine::Bing => format!("https://www.bing.com/search?q={e}"),
        Engine::Amazon => format!("https://www.amazon.com/s?k={e}"),
        Engine::Twitter => format!("https://twitter.com/search?q={e}"),
        Engine::GitHub => format!("https://github.com/search?q={e}"),
        Engine::EBay => format!("https://www.ebay.com/sch/i.html?_nkw={e}"),
        Engine::DuckDuckGo => format!("https://duckduckgo.com/?q={e}"),
        Engine::Perplexity => format!("https://www.perplexity.ai/search?q={e}"),
    }
}

// Port of your getBangUrl() logic.
fn bang_redirect(query: &str) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let first = words.first().copied().unwrap_or("");
    let last = words.last().copied().unwrap_or("");

    let mut bang: Option<&str> = None;
    let mut search: Option<String> = None;

    // suffix bang
    if last.starts_with('!') {
        bang = Some(last);
        search = Some(words[..words.len().saturating_sub(1)].join(" "));
    }
    // prefix bang overrides
    if first.starts_with('!') {
        bang = Some(first);
        search = Some(words.iter().skip(1).cloned().collect::<Vec<_>>().join(" "));
    }

    let Some(bang_str) = bang else { return None; };

    // "!" lucky case -> DuckDuckGo with the entire query
    if bang_str == "!" {
        return Some(engine_url(Engine::DuckDuckGo, trimmed));
    }

    // find engine for bang
    if let Some((_, engine)) = BANG_MAP.iter().find(|(b, _)| *b == bang_str) {
        // ensure we have some search text; if not, fall back to DDG with full query
        if let Some(s) = search {
            if !s.trim().is_empty() {
                return Some(engine_url(*engine, &s));
            }
        }
        return Some(engine_url(Engine::DuckDuckGo, trimmed));
    }

    // unknown bang -> default to DDG with entire query
    Some(engine_url(Engine::DuckDuckGo, trimmed))
}

fn redirect_response(to: &str) -> Response<Cursor<Vec<u8>>> {
    let loc = Header::from_bytes(&b"Location"[..], to.as_bytes()).unwrap();
    let mut body = Cursor::new(Vec::<u8>::new());
    // Optional tiny body for non-automatic clients
    body.get_mut().extend_from_slice(b"");
    Response::new(StatusCode(302), vec![loc], body, None, None)
}

fn handle_request(path: &str, raw_query: Option<&str>) -> Response<Cursor<Vec<u8>>> {
    // Extract q=...
    let mut q: Option<String> = None;
    if let Some(qs) = raw_query {
        for (k, v) in form_urlencoded::parse(qs.as_bytes()) {
            if k == "q" {
                q = Some(v.into_owned());
                break;
            }
        }
    }
    let query = match q {
        Some(s) if !s.trim().is_empty() => s,
        _ => return Response::from_string("Missing q").with_status_code(StatusCode(400)),
    };

    let log_search = |engine: Engine| {
        let now = Local::now();
        let truncated_query = if query.len() > 100 {
            format!("{}...", &query[..97])
        } else {
            query.clone()
        };
        eprintln!("[{}] Search: {} - \"{}\"", now.format("%Y-%m-%d %H:%M:%S"), engine.to_string(), truncated_query);
    };

    // Try bangs first
    if let Some(url) = bang_redirect(&query) {
        // Note: For bang redirects, the actual engine used is determined within the bang_redirect function
        // We'll log the default DuckDuckGo engine for unknown bangs
        let engine = if let Some((_, e)) = BANG_MAP.iter().find(|(b, _)| {
            let words: Vec<&str> = query.split_whitespace().collect();
            let first = words.first().copied().unwrap_or("");
            let last = words.last().copied().unwrap_or("");
            *b == first || *b == last
        }) {
            *e
        } else {
            Engine::DuckDuckGo
        };
        log_search(engine);
        return redirect_response(&url);
    }

    // Optional path service override: /google?q=... or /search/google?q=...
    let trimmed_path = path.trim_start_matches('/');
    let service = if trimmed_path.starts_with("search/") {
        trimmed_path.trim_start_matches("search/").trim()
    } else {
        trimmed_path.trim()
    };
    if !service.is_empty() {
        let engine = match service.to_ascii_lowercase().as_str() {
            "google" => Some(Engine::Google),
            "youtube" => Some(Engine::YouTube),
            "wikipedia" | "wiki" => Some(Engine::Wikipedia),
            "claude" => Some(Engine::Claude),
            "chatgpt" | "gpt" | "chat" => Some(Engine::ChatGPT),
            "images" | "gi" => Some(Engine::GoogleImages),
            "wolfram" | "wolframalpha" | "wa" => Some(Engine::Wolfram),
            "reddit" | "r" => Some(Engine::Reddit),
            "bing" | "b" => Some(Engine::Bing),
            "amazon" | "a" => Some(Engine::Amazon),
            "twitter" | "x" | "tw" => Some(Engine::Twitter),
            "github" | "gh" => Some(Engine::GitHub),
            "ebay" => Some(Engine::EBay),
            "ddg" | "duckduckgo" => Some(Engine::DuckDuckGo),
            "perplexity" | "p" => Some(Engine::Perplexity),
            _ => None,
        };
        if let Some(e) = engine {
            log_search(e);
            return redirect_response(&engine_url(e, &query));
        }
        return Response::from_string("Unknown service").with_status_code(StatusCode(404));
    }

    // No bang and no explicit service: go to default engine (Perplexity)
    log_search(DEFAULT_ENGINE);
    redirect_response(&engine_url(DEFAULT_ENGINE, &query))
}

fn main() {
    // Bind address
    let port: u16 = std::env::var("SIDFREY_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7777);
    let addr = format!("127.0.0.1:{port}");
    let socket = addr
        .to_socket_addrs()
        .expect("invalid addr")
        .next()
        .expect("no socket addrs");
    let server = Server::http(socket).expect("failed to bind");

    eprintln!("sidfrey-router listening on http://{addr}");

    for rq in server.incoming_requests() {
        let method = rq.method().clone();
        let url = rq.url().to_string(); // e.g., "/perplexity?q=hello" or "/?q=hello"
        let (path, query) = match url.split_once('?') {
            Some((p, q)) => (p, Some(q)),
            None => (url.as_str(), None),
        };

        if method == Method::Get || method == Method::Head {
            let resp = handle_request(path, query);
            let _ = rq.respond(resp);
        } else {
            let _ = rq.respond(Response::from_string("Method Not Allowed").with_status_code(StatusCode(405)));
        }
    }
}
