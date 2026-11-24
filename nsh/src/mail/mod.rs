//! Mail accounts and a fixture IMAP source: headers first, bodies on demand,
//! no read flags, attachments or remote images in what the UI receives.

mod credentials;
mod imap;

use std::env;

use serde::{Deserialize, Serialize};

pub use credentials::{delete_secret, save_secret};
pub use imap::ImapSource;

pub const MAX_ACCOUNTS: usize = 5;
const DEFAULT_IMAP_PORT: u16 = 993;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub email: String,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Provider {
    pub id: &'static str,
    pub label: &'static str,
    pub host: &'static str,
}

pub const PROVIDERS: [Provider; 6] = [
    Provider { id: "qq", label: "QQ 邮箱", host: "imap.qq.com" },
    Provider { id: "163", label: "网易 163", host: "imap.163.com" },
    Provider { id: "126", label: "网易 126", host: "imap.126.com" },
    Provider { id: "gmail", label: "Gmail", host: "imap.gmail.com" },
    Provider { id: "outlook", label: "Outlook", host: "outlook.office365.com" },
    Provider { id: "custom", label: "自定义", host: "" },
];

pub fn provider_for(email: &str) -> &'static Provider {
    let domain = email.split_once('@').map_or("", |(_, domain)| domain);
    if matches!(domain, "qq.com" | "foxmail.com" | "vip.qq.com") {
        return &PROVIDERS[0];
    }
    if domain == "163.com" {
        return &PROVIDERS[1];
    }
    if domain == "126.com" {
        return &PROVIDERS[2];
    }
    if domain == "gmail.com" {
        return &PROVIDERS[3];
    }
    if matches!(domain, "outlook.com" | "hotmail.com" | "live.com" | "office365.com") {
        return &PROVIDERS[4];
    }
    &PROVIDERS[5]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailError {
    Unconfigured,
    AuthFailed,
    Timeout,
    Offline(String),
    ReadFailed(String),
}

impl MailError {
    pub fn message(&self) -> String {
        match self {
            Self::Unconfigured => "尚未配置邮箱账户。".to_string(),
            Self::AuthFailed => "认证失败，请检查邮箱地址和授权码。".to_string(),
            Self::Timeout => "连接邮箱超时，请检查网络后重试。".to_string(),
            Self::Offline(error) => format!("无法连接邮箱：{error}"),
            Self::ReadFailed(error) => format!("读取邮件失败：{error}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Unconfigured,
    Saved,
    Ready,
    AuthFailed,
    Timeout,
    Offline,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub id: String,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub unread: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub status: Status,
    pub headers: Vec<Header>,
    pub body: String,
    pub body_id: Option<String>,
    pub error: Option<String>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            status: Status::Unconfigured,
            headers: Vec::new(),
            body: String::new(),
            body_id: None,
            error: None,
        }
    }
}

pub trait Source: Send {
    fn headers(&mut self) -> Result<Vec<Header>, MailError>;
    fn body(&mut self, id: &str) -> Result<String, MailError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixtureKind {
    Ok,
    Auth,
    Timeout,
    Offline,
    ReadFailed,
    Unavailable,
}

pub struct Fixture {
    kind: FixtureKind,
    body_fetches: usize,
}

impl Fixture {
    pub fn from_environment() -> Self {
        let kind = match env::var("NSH_MAIL_FIXTURE").unwrap_or_default().as_str() {
            "ok" => FixtureKind::Ok,
            "auth" => FixtureKind::Auth,
            "timeout" => FixtureKind::Timeout,
            "offline" => FixtureKind::Offline,
            "error" => FixtureKind::ReadFailed,
            _ => FixtureKind::Unavailable,
        };
        Self { kind, body_fetches: 0 }
    }

    #[cfg(test)]
    pub fn ok() -> Self {
        Self { kind: FixtureKind::Ok, body_fetches: 0 }
    }
}

impl Source for Fixture {
    fn headers(&mut self) -> Result<Vec<Header>, MailError> {
        match self.kind {
            FixtureKind::Ok => Ok(vec![
                Header {
                    id: "101".into(),
                    from: "团队".into(),
                    subject: "本周进度".into(),
                    date: "11月24日 09:30".into(),
                    unread: true,
                },
                Header {
                    id: "102".into(),
                    from: "账单".into(),
                    subject: "电子发票".into(),
                    date: "11月23日 18:10".into(),
                    unread: false,
                },
            ]),
            FixtureKind::Auth => Err(MailError::AuthFailed),
            FixtureKind::Timeout => Err(MailError::Timeout),
            FixtureKind::Offline => Err(MailError::Offline("网络不可达".into())),
            FixtureKind::ReadFailed => Err(MailError::ReadFailed("夹具读取失败".into())),
            FixtureKind::Unavailable => Err(MailError::Offline("真实 IMAP 尚未接入".into())),
        }
    }

    fn body(&mut self, id: &str) -> Result<String, MailError> {
        self.body_fetches += 1;
        match self.kind {
            FixtureKind::Ok => Ok(format!(
                "<p>邮件 {id} 的正文。</p><img src=\"https://track.example/pixel.png\"><script>alert(1)</script><p>第二段</p>--attachment--\ninvoice.pdf"
            )),
            FixtureKind::Auth => Err(MailError::AuthFailed),
            FixtureKind::Timeout => Err(MailError::Timeout),
            FixtureKind::Offline => Err(MailError::Offline("网络不可达".into())),
            FixtureKind::ReadFailed => Err(MailError::ReadFailed("夹具读取失败".into())),
            FixtureKind::Unavailable => Err(MailError::Offline("真实 IMAP 尚未接入".into())),
        }
    }
}

pub struct Service {
    accounts: Vec<Account>,
    source: Box<dyn Source>,
    snapshot: Snapshot,
}

impl Service {
    pub fn new(accounts: Vec<Account>) -> Self {
        let source = source_for(&accounts);
        let mut service = Self {
            accounts,
            source,
            snapshot: Snapshot::default(),
        };
        service.refresh_status();
        service
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.clone()
    }

    pub fn set_accounts(&mut self, accounts: Vec<Account>) {
        self.accounts = accounts;
        self.source = source_for(&self.accounts);
        self.snapshot = Snapshot::default();
        self.refresh_status();
    }

    pub fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    #[cfg(test)]
    pub fn upsert(&mut self, account: Account) -> Result<(), String> {
        if !self.accounts.iter().any(|current| current.email == account.email)
            && self.accounts.len() >= MAX_ACCOUNTS
        {
            return Err(format!("最多保存 {MAX_ACCOUNTS} 个邮箱。"));
        }
        match self.accounts.iter_mut().find(|current| current.email == account.email) {
            Some(current) => *current = account,
            None => self.accounts.push(account),
        }
        self.refresh_status();
        Ok(())
    }


    fn refresh_status(&mut self) {
        if self.accounts.is_empty() {
            self.snapshot = Snapshot::default();
        } else if self.snapshot.status == Status::Unconfigured {
            self.snapshot.status = Status::Saved;
        }
    }

    pub fn probe(&mut self) -> Snapshot {
        if self.accounts.is_empty() {
            self.fail(MailError::Unconfigured);
            return self.snapshot();
        }
        match self.source.headers() {
            Ok(headers) => {
                self.snapshot.status = Status::Ready;
                self.snapshot.headers = headers;
                self.snapshot.error = None;
            }
            Err(error) => self.fail(error),
        }
        self.snapshot()
    }

    pub fn body(&mut self, id: &str) -> Snapshot {
        match self.source.body(id) {
            Ok(body) => {
                self.snapshot.body = sanitize_body(&body);
                self.snapshot.body_id = Some(id.to_string());
                self.snapshot.error = None;
            }
            Err(error) => self.fail(error),
        }
        self.snapshot()
    }

    fn fail(&mut self, error: MailError) {
        self.snapshot.status = match error {
            MailError::AuthFailed => Status::AuthFailed,
            MailError::Timeout => Status::Timeout,
            MailError::Offline(_) => Status::Offline,
            MailError::ReadFailed(_) => Status::Error,
            MailError::Unconfigured => Status::Unconfigured,
        };
        self.snapshot.error = Some(error.message());
    }
}

fn source_for(accounts: &[Account]) -> Box<dyn Source> {
    if env::var_os("NSH_MAIL_FIXTURE").is_some() {
        return Box::new(Fixture::from_environment());
    }
    match accounts.first() {
        Some(account) => Box::new(ImapSource::new(account.clone())),
        None => Box::new(Fixture::from_environment()),
    }
}

pub fn normalize_email(email: &str) -> Result<String, String> {
    let email = email.trim().to_ascii_lowercase();
    let valid = email.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && local.len() <= 64
            && local
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
            && is_domain(domain)
    });
    if !valid {
        return Err("请输入有效的邮箱地址。".into());
    }
    Ok(email)
}

fn is_domain(value: &str) -> bool {
    value.len() <= 253
        && value.contains('.')
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

pub fn resolve_host(email: &str, host: &str) -> Result<String, String> {
    let host = host.trim().to_ascii_lowercase();
    let host = if host.is_empty() {
        provider_for(email).host.to_string()
    } else {
        host
    };
    let host = if host.is_empty() {
        let domain = email.split_once('@').map_or("", |(_, domain)| domain);
        format!("imap.{domain}")
    } else {
        host
    };
    let (name, port) = match host.split_once(':') {
        Some((name, port)) => {
            let port = port
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| "IMAP 端口无效。".to_string())?;
            (name, port)
        }
        None => (host.as_str(), DEFAULT_IMAP_PORT),
    };
    if !is_domain(name) {
        return Err("IMAP 服务器地址无效。".into());
    }
    Ok(if port == DEFAULT_IMAP_PORT {
        name.to_string()
    } else {
        format!("{name}:{port}")
    })
}

pub fn sanitize_body(raw: &str) -> String {
    let without_attachments = raw.split("--attachment--").next().unwrap_or(raw);
    let mut out = String::new();
    let mut rest = without_attachments;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let end = start + end;
        let tag = rest[start + 1..end].trim().to_ascii_lowercase();
        let name = tag.split_whitespace().next().unwrap_or("");
        if name == "br" || name == "/p" || name == "/div" || name == "/li" {
            out.push('\n');
        }
        rest = &rest[end + 1..];
        if name == "script" || name == "style" {
            let close = format!("</{name}>");
            if let Some(close_at) = rest.to_ascii_lowercase().find(&close) {
                rest = &rest[close_at + close.len()..];
            }
        }
    }
    out.push_str(rest);
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn email(local: &str, domain: &str) -> String {
        format!("{local}@{domain}")
    }

    fn account(address: &str) -> Account {
        Account { email: address.into(), host: String::new(), enabled: true }
    }

    #[test]
    fn addresses_and_provider_hosts_are_normalized() {
        assert_eq!(normalize_email(&email("user", "example.com")).unwrap(), email("user", "example.com"));
        assert!(normalize_email("not an email").is_err());
        assert_eq!(provider_for(&email("user", "qq.com")).id, "qq");
        assert_eq!(provider_for(&email("user", "outlook.com")).id, "outlook");
        assert_eq!(resolve_host(&email("user", "example.com"), "").unwrap(), "imap.example.com");
        assert_eq!(resolve_host(&email("user", "qq.com"), "imap.qq.com:993").unwrap(), "imap.qq.com");
        assert_eq!(resolve_host(&email("user", "qq.com"), "imap.qq.com:1993").unwrap(), "imap.qq.com:1993");
    }

    #[test]
    fn headers_are_fetched_without_bodies_and_the_body_is_on_demand() {
        let mut service = Service::new(vec![account(&email("user", "example.com"))]);
        service.source = Box::new(Fixture::ok());
        let snapshot = service.probe();
        assert_eq!(snapshot.status, Status::Ready);
        assert_eq!(snapshot.headers.len(), 2);
        assert!(snapshot.body.is_empty());
        let body = service.body("101");
        assert_eq!(body.body_id.as_deref(), Some("101"));
        assert!(body.body.contains("邮件 101"));
    }

    #[test]
    fn the_body_drops_remote_images_scripts_and_attachments() {
        let text = sanitize_body(
            "<p>hello</p><img src=\"https://track.example/a.png\"><script>bad()</script><p>world</p>--attachment--\ninvoice.pdf",
        );
        assert_eq!(text, "hello\nworld");
        assert!(!text.contains("http"));
        assert!(!text.contains("invoice"));
    }

    #[test]
    fn failure_states_are_distinct_and_retryable() {
        let mut service = Service::new(vec![account(&email("user", "example.com"))]);
        let mut fixture = Fixture::from_environment();
        fixture.kind = FixtureKind::Auth;
        service.source = Box::new(fixture);
        assert_eq!(service.probe().status, Status::AuthFailed);
        let mut fixture = Fixture::from_environment();
        fixture.kind = FixtureKind::Timeout;
        service.source = Box::new(fixture);
        assert_eq!(service.probe().status, Status::Timeout);
        let mut fixture = Fixture::from_environment();
        fixture.kind = FixtureKind::Offline;
        service.source = Box::new(fixture);
        assert_eq!(service.probe().status, Status::Offline);
        service.source = Box::new(Fixture::ok());
        assert_eq!(service.probe().status, Status::Ready);
    }

    #[test]
    fn accounts_are_unique_and_bounded() {
        let mut service = Service::new(Vec::new());
        for index in 0..MAX_ACCOUNTS {
            service.upsert(account(&email(&format!("user{index}"), "example.com"))).unwrap();
        }
        assert!(service.upsert(account(&email("extra", "example.com"))).is_err());
        service.upsert(account(&email("user0", "example.com"))).unwrap();
        assert_eq!(service.accounts().len(), MAX_ACCOUNTS);
    }

    #[test]
    fn the_empty_account_list_is_unconfigured() {
        let service = Service::new(Vec::new());
        assert_eq!(service.snapshot().status, Status::Unconfigured);
    }
}
