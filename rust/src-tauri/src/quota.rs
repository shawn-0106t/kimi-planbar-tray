// Quota fetch + defensive parsing, 1:1 port of QuotaService (SPEC section 16).
// Traps honored here:
//  - server JSON numbers are modeled as strings, numeric fallback tolerated
//  - Extra Usage amountLeft unit is 1e-8 yuan -> cents = (raw + 500000) / 1000000
//  - isEnabled == false must be reported as NotActivated (KimiCodeBar v1.1.1 bug)

use chrono::{DateTime, Local};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;

use crate::credentials;

const USAGES_URL: &str = "https://api.kimi.com/coding/v1/usages";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSegment {
    pub percent: f64,
    pub reset_at: Option<DateTime<Local>>,
}

#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ExtraState {
    NotActivated,
    NoData,
    Ready,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraInfo {
    pub state: ExtraState,
    pub balance_cents: Option<i64>,
    pub monthly_enabled: bool,
    pub monthly_used_cents: Option<i64>,
    pub monthly_limit_cents: Option<i64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaResult {
    pub five_hour: Option<QuotaSegment>,
    pub week: Option<QuotaSegment>,
    pub extra: Option<ExtraInfo>,
    pub fetched_at: DateTime<Local>,
    pub error: Option<String>,
}

impl QuotaResult {
    fn failed(kind: &str) -> Self {
        QuotaResult {
            five_hour: None,
            week: None,
            extra: None,
            fetched_at: Local::now(),
            error: Some(kind.to_string()),
        }
    }

    /// On failure keep last-known-good data: fill null fields from `last`
    /// (SPEC 16.5 step 2; the UI only shows the failure hint in the status line).
    pub fn fill_missing_from(&mut self, last: &QuotaResult) {
        if self.five_hour.is_none() {
            self.five_hour = last.five_hour.clone();
        }
        if self.week.is_none() {
            self.week = last.week.clone();
        }
        if self.extra.is_none() {
            self.extra = last.extra.clone();
        }
    }
}

fn http_client() -> Option<&'static Client> {
    static CLIENT: OnceLock<Option<Client>> = OnceLock::new();
    CLIENT
        .get_or_init(|| Client::builder().timeout(Duration::from_secs(10)).build().ok())
        .as_ref()
}

pub async fn fetch() -> QuotaResult {
    let Some(token) = credentials::load_token() else {
        return QuotaResult::failed("no-token");
    };
    let Some(client) = http_client() else {
        return QuotaResult::failed("HttpRequestException");
    };
    let resp = match client
        .get(USAGES_URL)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // Keep .NET exception-type names so --test-fetch output matches the WPF reference
            return QuotaResult::failed(if e.is_timeout() {
                "TaskCanceledException"
            } else {
                "HttpRequestException"
            });
        }
    };
    if !resp.status().is_success() {
        return QuotaResult::failed("HttpRequestException");
    }
    let root: Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return QuotaResult::failed("JsonException"),
    };

    let mut r = QuotaResult {
        five_hour: None,
        week: None,
        extra: None,
        fetched_at: Local::now(),
        error: None,
    };
    // 5-hour segment: root.limits[0].detail
    if let Some(detail) = root
        .get("limits")
        .and_then(|l| l.as_array())
        .and_then(|a| a.first())
        .and_then(|first| first.get("detail"))
    {
        r.five_hour = Some(parse_segment(detail));
    }
    // Week segment: root.usage
    if let Some(u) = root.get("usage") {
        if u.is_object() {
            r.week = Some(parse_segment(u));
        }
    }
    r.extra = Some(parse_extra(root.get("boosterWallet")));
    r
}

/// JSON number-or-string -> f64, missing -> 0.
fn get_f64(v: &Value, key: &str) -> f64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// JSON number-or-string -> i64.
fn get_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

/// resetTime: RFC3339 first, then the looser shapes DateTimeOffset.TryParse
/// accepts (space separator, with/without offset; no offset = local time).
fn parse_reset_time(s: &str) -> Option<DateTime<Local>> {
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Local));
    }
    for fmt in [
        "%Y-%m-%d %H:%M:%S %:z",
        "%Y-%m-%d %H:%M:%S %z",
        "%Y-%m-%dT%H:%M:%S %:z",
        "%Y-%m-%d %H:%M:%S%.f %:z",
    ] {
        if let Ok(d) = DateTime::parse_from_str(s, fmt) {
            return Some(d.with_timezone(&Local));
        }
    }
    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(n) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            if let Some(d) = n.and_local_timezone(Local).single() {
                return Some(d);
            }
        }
    }
    None
}

fn parse_segment(v: &Value) -> QuotaSegment {
    let used = get_f64(v, "used");
    let mut limit = get_f64(v, "limit");
    if limit <= 0.0 {
        limit = 1.0; // guard against division by zero
    }
    let reset_at = v
        .get("resetTime")
        .and_then(|x| x.as_str())
        .and_then(parse_reset_time);
    QuotaSegment {
        percent: used / limit * 100.0,
        reset_at,
    }
}

fn parse_cents(money: Option<&Value>) -> Option<i64> {
    money
        .filter(|m| m.is_object())
        .and_then(|m| m.get("priceInCents"))
        .and_then(get_i64)
}

fn parse_extra(wallet: Option<&Value>) -> ExtraInfo {
    let mut info = ExtraInfo {
        state: ExtraState::NotActivated,
        balance_cents: None,
        monthly_enabled: false,
        monthly_used_cents: None,
        monthly_limit_cents: None,
    };
    let Some(w) = wallet.filter(|w| w.is_object()) else {
        return info; // not an object -> NotActivated
    };

    // isEnabled defense: when the booster is disabled, amountLeft is an estimate
    // (monthly limit minus used), NOT the real balance -> must read as NotActivated.
    if w.get("isEnabled").and_then(|v| v.as_bool()) == Some(false) {
        return info;
    }

    if let Some(raw) = w
        .get("balance")
        .filter(|b| b.is_object())
        .and_then(|b| b.get("amountLeft"))
        .and_then(get_i64)
    {
        info.state = ExtraState::Ready;
        info.balance_cents = Some((raw + 500_000) / 1_000_000); // 1e-8 yuan -> cents, rounded
    } else {
        info.state = ExtraState::NoData;
    }

    if w.get("monthlyChargeLimitEnabled").and_then(|v| v.as_bool()) == Some(true) {
        info.monthly_enabled = true;
        info.monthly_used_cents = parse_cents(w.get("monthlyUsed"));
        info.monthly_limit_cents = parse_cents(w.get("monthlyChargeLimit"));
    }
    info
}
