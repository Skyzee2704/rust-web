//! Server-rendered pages. Markup lives in `templates/` and uses the same class names as the Next.js app.

use askama::Template;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use chrono::Datelike;
use serde::Deserialize;

use crate::auth;
use crate::config::{self, Task, SITE};
use crate::state::AppState;
use crate::util::{css_delay, format_int, mint_when};

struct Shell {
    title: String,
    description: String,
    og_description: String,
    site_name: &'static str,
    site_name_upper: String,
    site_url: String,
    og_image: String,
    x_handle: &'static str,
    x_url: &'static str,
    share_host: &'static str,
    year: i32,
    show_cursor: bool,
    has_opensea: bool,
    opensea_url: &'static str,
}

fn shell(state: &AppState, title: &str, description: &str, show_cursor: bool) -> Shell {
    Shell {
        title: title.to_string(),
        description: description.to_string(),
        og_description: SITE.description.to_string(),
        site_name: SITE.name,
        site_name_upper: SITE.name.to_uppercase(),
        site_url: state.site_url.clone(),
        og_image: format!("{}/logo.png", state.site_url),
        x_handle: SITE.x_handle,
        x_url: SITE.x_url,
        share_host: state.share_host,
        year: chrono::Utc::now().year(),
        show_cursor,
        has_opensea: !SITE.opensea.is_empty(),
        opensea_url: SITE.opensea,
    }
}

fn render(t: impl Template, status: StatusCode, private: bool) -> Response {
    match t.render() {
        Ok(body) => {
            let mut res = (status, Html(body)).into_response();
            if private {
                res.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("private, no-store"),
                );
            }
            res
        }
        Err(e) => {
            tracing::error!("template: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}

struct Letter {
    ch: char,
    delay: String,
}
struct Petal {
    left: String,
    delay: String,
    dur: String,
    scale: String,
}
struct HeroNft {
    image: &'static str,
    color: &'static str,
    rot: i32,
    delay: String,
    bob: String,
    klass: &'static str,
}
struct Stat {
    k: &'static str,
    v: String,
    delay: String,
}
struct MarqueeBit {
    word: &'static str,
    spark: &'static str,
}
struct Word {
    text: &'static str,
    delay: String,
}
struct ChapterView {
    num: String,
    title: &'static str,
    body: Vec<&'static str>,
}
struct StepView {
    num: String,
    title: &'static str,
    body: &'static str,
    delay: String,
}
struct Kv {
    k: String,
    v: String,
}
struct Sleeper {
    rot: i32,
    delay: &'static str,
    y: i32,
    klass: &'static str,
}
struct FaqView {
    q: &'static str,
    a: &'static str,
    delay: String,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage {
    shell: Shell,
    letters: Vec<Letter>,
    petals: Vec<Petal>,
    count_pad: String,
    supply_n: u32,
    supply_label: String,
    tagline: &'static str,
    hero_nfts: Vec<HeroNft>,
    stats: Vec<Stat>,
    marquee: Vec<MarqueeBit>,
    lore_words: Vec<Word>,
    chapters: Vec<ChapterView>,
    legend_total: String,
    legend_index: String,
    legend_badge: String,
    legend_alt: String,
    legend_image: &'static str,
    legend_color: &'static str,
    legend_shadow: String,
    legends_json: String,
    unrevealed: u32,
    sleepers: Vec<Sleeper>,
    join_words: Vec<Word>,
    steps: Vec<StepView>,
    mint_announced: bool,
    mint_iso: String,
    mint_rows: Vec<Kv>,
    mint_departure: String,
    bars: Vec<i32>,
    faq_words: Vec<Word>,
    faqs: Vec<FaqView>,
    first_name: &'static str,
    first_image: &'static str,
    first_color: &'static str,
}

pub async fn home(State(state): State<AppState>) -> Response {
    render(home_page(&state), StatusCode::OK, false)
}

fn home_page(state: &AppState) -> HomePage {
    let mint = mint_when(SITE.mint_date);
    let letters = SITE
        .name
        .to_uppercase()
        .chars()
        .enumerate()
        .map(|(i, ch)| Letter {
            ch,
            delay: css_delay(100 + i as i32 * 70),
        })
        .collect();
    let petals = (0..14)
        .map(|i| {
            let delay = trim_float((i as f64 * 1.7) % 12.0);
            let scale = trim_float(0.6 + ((i * 7) % 5) as f64 / 8.0);
            Petal {
                left: format!("{}%", (i * 37) % 100),
                delay: format!("{delay}s"),
                dur: format!("{}s", 10 + ((i * 3) % 8)),
                scale,
            }
        })
        .collect();
    let rots = [-12, -4, 5, 12];
    let hero_nfts = config::LEGENDS
        .iter()
        .take(4)
        .enumerate()
        .map(|(i, l)| HeroNft {
            image: l.image,
            color: l.color,
            rot: rots[i],
            delay: css_delay(700 + i as i32 * 120),
            bob: css_delay(i as i32 * 600),
            klass: if i == 1 || i == 2 {
                "-mx-2 z-10 -mt-3"
            } else {
                "-mx-2 mt-2"
            },
        })
        .collect();
    let stats = vec![
        Stat {
            k: "Supply",
            v: format_int(SITE.supply as u64),
            delay: css_delay(1000),
        },
        Stat {
            k: "Chain",
            v: SITE.chain.to_string(),
            delay: css_delay(1120),
        },
        Stat {
            k: "Mint",
            v: mint.label.clone(),
            delay: css_delay(1240),
        },
    ];
    let mut marquee = Vec::new();
    for i in 0..(config::MARQUEE.len() * 4) {
        marquee.push(MarqueeBit {
            word: config::MARQUEE[i % config::MARQUEE.len()],
            spark: if i % 2 == 1 {
                "h-4 w-4 text-mint-pale"
            } else {
                "h-4 w-4 text-sakura"
            },
        });
    }
    let chapters = config::LORE
        .iter()
        .enumerate()
        .map(|(i, c)| ChapterView {
            num: format!("{:02}", i + 1),
            title: c.title,
            body: c.body.to_vec(),
        })
        .collect();
    let steps = config::STEPS
        .iter()
        .enumerate()
        .map(|(i, s)| StepView {
            num: format!("{:02}", i + 1),
            title: s.title,
            body: s.body,
            delay: css_delay(i as i32 * 80),
        })
        .collect();
    let phases = SITE.phases.join(" → ");
    let mint_rows = vec![
        Kv {
            k: "Supply".into(),
            v: format_int(SITE.supply as u64),
        },
        Kv {
            k: "Chain".into(),
            v: SITE.chain.into(),
        },
        Kv {
            k: "WL price".into(),
            v: SITE.wl_price.into(),
        },
        Kv {
            k: "Public".into(),
            v: SITE.mint_price.into(),
        },
        Kv {
            k: "Phases".into(),
            v: phases,
        },
    ];
    let bars = (0..44).map(|i| ((i * 7919) % 5) + 1).collect();
    let sleepers = [
        (-8, "0s", 10, ""),
        (5, "1.6s", 0, ""),
        (-3, "3.1s", 14, ""),
        (9, "0.8s", 4, "hidden sm:block"),
        (-6, "2.4s", 12, "hidden sm:block"),
    ]
    .into_iter()
    .map(|(rot, delay, y, klass)| Sleeper {
        rot,
        delay,
        y,
        klass,
    })
    .collect();
    let faqs = config::FAQS
        .iter()
        .enumerate()
        .map(|(i, f)| FaqView {
            q: f.q,
            a: f.a,
            delay: css_delay((i.min(4) as i32) * 40),
        })
        .collect();
    let first = &config::LEGENDS[0];
    let items: Vec<_> = config::LEGENDS
        .iter()
        .map(|l| serde_json::json!({"image": l.image, "color": l.color, "name": l.name}))
        .collect();
    let legends_json = serde_json::Value::Array(items)
        .to_string()
        .replace('<', "\\u003c");
    let width = SITE.supply.to_string().len();
    HomePage {
        shell: shell(
            state,
            &format!("{} — {}", SITE.name, SITE.tagline),
            SITE.description,
            true,
        ),
        letters,
        petals,
        count_pad: "0".repeat(width),
        supply_n: SITE.supply,
        supply_label: format_int(SITE.supply as u64),
        tagline: SITE.tagline,
        hero_nfts,
        stats,
        marquee,
        lore_words: split_words("Born from the river of forgetting."),
        chapters,
        legend_total: format!("{:02}", config::LEGENDS.len()),
        legend_index: "01".into(),
        legend_badge: "001".into(),
        legend_alt: "Lethean #001 — sample".into(),
        legend_image: first.image,
        legend_color: first.color,
        legend_shadow: format!("0 24px 60px -18px {c}, 0 0 0 6px {c}33", c = first.color),
        legends_json,
        unrevealed: config::unrevealed(),
        sleepers,
        join_words: split_words("Four steps before you drink."),
        steps,
        mint_announced: mint.announced,
        mint_iso: mint.iso,
        mint_rows,
        mint_departure: mint.departure,
        bars,
        faq_words: split_words("Questions from the riverbank."),
        faqs,
        first_name: first.name,
        first_image: first.image,
        first_color: first.color,
    }
}

fn split_words(s: &'static str) -> Vec<Word> {
    s.split(' ')
        .enumerate()
        .map(|(i, text)| Word {
            text,
            delay: css_delay(i as i32 * 80),
        })
        .collect()
}

fn trim_float(n: f64) -> String {
    let s = format!("{n:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() {
        "0".into()
    } else {
        s.to_string()
    }
}

struct Pip {
    label: &'static str,
    mark: String,
    klass: &'static str,
    current: bool,
}
struct HowRow {
    num: String,
    text: &'static str,
}

#[derive(Template)]
#[template(path = "apply.html")]
struct ApplyPage {
    shell: Shell,
    demo_mode: bool,
    auth_configured: bool,
    signed_in: bool,
    username: String,
    show_checking: bool,
    has_auth_error: bool,
    auth_error: String,
    task_count: usize,
    required_count: usize,
    unrevealed: String,
    supply_label: String,
    chain: &'static str,
    first_image: &'static str,
    first_name: &'static str,
    first_color: &'static str,
    tasks: &'static [Task],
    how: Vec<HowRow>,
    pips: Vec<Pip>,
    quests_locked: bool,
}

#[derive(Deserialize)]
pub struct ErrQuery {
    error: Option<String>,
}

pub async fn apply_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ErrQuery>,
) -> Response {
    let session = auth::session_user(&state, &headers);
    let signed_in = session.is_some();
    let username = session.map(|s| s.username).unwrap_or_default();
    let auth_error = match q.error.as_deref() {
        Some("access_denied") => "X sign-in was cancelled. You can try again.".to_string(),
        Some(s) if !s.is_empty() => "X sign-in didn't complete. Please try again.".to_string(),
        _ => String::new(),
    };
    let step = if signed_in { 1 } else { 0 };
    let labels = ["X account", "Quests", "Wallet", "Done"];
    let pips = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let (mark, klass) = if i < step {
                ("✓".to_string(), "border-mint bg-mint text-white")
            } else if i == step {
                (
                    (i + 1).to_string(),
                    "border-peach text-peach-deep shadow-[0_0_16px_rgba(239,122,69,0.45)]",
                )
            } else {
                ((i + 1).to_string(), "border-sky/25 text-current/40")
            };
            Pip {
                label,
                mark,
                klass,
                current: i == step,
            }
        })
        .collect();
    let how = config::APPLY_HOW
        .iter()
        .enumerate()
        .map(|(i, text)| HowRow {
            num: format!("{:02}", i + 1),
            text,
        })
        .collect();
    let required_count = config::TASKS.iter().filter(|t| t.required).count();
    let page = ApplyPage {
        shell: shell(
            &state,
            "Apply for whitelist · Letheans",
            "Connect X, finish the quests, submit your wallet.",
            false,
        ),
        demo_mode: state.demo_mode,
        auth_configured: state.auth_configured,
        signed_in,
        username,
        show_checking: signed_in && !state.demo_mode,
        has_auth_error: !auth_error.is_empty(),
        auth_error,
        task_count: config::TASKS.len(),
        required_count,
        unrevealed: config::unrevealed().to_string(),
        supply_label: format_int(SITE.supply as u64),
        chain: SITE.chain,
        first_image: config::LEGENDS[0].image,
        first_name: config::LEGENDS[0].name,
        first_color: config::LEGENDS[0].color,
        tasks: config::TASKS,
        how,
        pips,
        quests_locked: !signed_in,
    };
    render(page, StatusCode::OK, true)
}

#[derive(Template)]
#[template(path = "status.html")]
struct StatusPage {
    shell: Shell,
}

pub async fn status_page(State(state): State<AppState>) -> Response {
    render(
        StatusPage {
            shell: shell(
                &state,
                "Check WL status · Letheans",
                "Paste your wallet or X username to see your whitelist card and status.",
                false,
            ),
        },
        StatusCode::OK,
        false,
    )
}

#[derive(Template)]
#[template(path = "not_found.html")]
struct NotFoundPage {
    shell: Shell,
}

pub fn not_found(state: &AppState) -> Response {
    render(
        NotFoundPage {
            shell: shell(
                state,
                "Page not found · Letheans",
                "This page could not be found.",
                false,
            ),
        },
        StatusCode::NOT_FOUND,
        false,
    )
}
