//! Minimal read-only IMAP over TLS: EXAMINE, header fetch, and body.part 1
//! on demand. No STORE, no attachment parts, no plaintext fallback.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::Arc,
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine};
use rustls::{
    pki_types::ServerName, ClientConfig, ClientConnection, RootCertStore, StreamOwned,
};

use super::{credentials, Account, Header, MailError, Source};

const IO_TIMEOUT: Duration = Duration::from_secs(12);
const HEADER_LIMIT: usize = 20;
static CRYPTO: std::sync::OnceLock<()> = std::sync::OnceLock::new();

pub struct ImapSource {
    account: Account,
}

impl ImapSource {
    pub fn new(account: Account) -> Self {
        Self { account }
    }
}

impl Source for ImapSource {
    fn headers(&mut self) -> Result<Vec<Header>, MailError> {
        let secret = credentials::read_secret(&self.account.email)
            .map_err(MailError::ReadFailed)?
            .ok_or(MailError::Unconfigured)?;
        fetch_headers(&self.account, &secret, HEADER_LIMIT)
    }

    fn body(&mut self, id: &str) -> Result<String, MailError> {
        let secret = credentials::read_secret(&self.account.email)
            .map_err(MailError::ReadFailed)?
            .ok_or(MailError::Unconfigured)?;
        fetch_body(&self.account, &secret, id)
    }
}

struct Session {
    reader: BufReader<StreamOwned<ClientConnection, TcpStream>>,
    tag: u32,
}

impl Session {
    fn connect(account: &Account, secret: &str) -> Result<Self, MailError> {
        let (name, port) = split_server(&account.host);
        let address = (name, port)
            .to_socket_addrs()
            .map_err(|error| MailError::Offline(error.to_string()))?
            .next()
            .ok_or_else(|| MailError::Offline("服务器地址无法解析".into()))?;
        let tcp = TcpStream::connect_timeout(&address, IO_TIMEOUT).map_err(|error| {
            if error.kind() == std::io::ErrorKind::TimedOut {
                MailError::Timeout
            } else {
                MailError::Offline(error.to_string())
            }
        })?;
        tcp.set_read_timeout(Some(IO_TIMEOUT)).ok();
        tcp.set_write_timeout(Some(IO_TIMEOUT)).ok();
        CRYPTO.get_or_init(|| {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        });
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let server_name = ServerName::try_from(name.to_string())
            .map_err(|error| MailError::Offline(format!("IMAP 主机名无效：{error}")))?;
        let connection = ClientConnection::new(Arc::new(config), server_name)
            .map_err(|error| MailError::Offline(format!("TLS 初始化失败：{error}")))?;
        let mut session = Self {
            reader: BufReader::new(StreamOwned::new(connection, tcp)),
            tag: 0,
        };
        session.read_greeting()?;
        session.login(&account.email, secret)?;
        Ok(session)
    }

    fn read_greeting(&mut self) -> Result<(), MailError> {
        let mut line = Vec::new();
        self.reader
            .read_until(b'\n', &mut line)
            .map_err(|error| MailError::Offline(error.to_string()))?;
        if String::from_utf8_lossy(&line).starts_with("* OK") {
            Ok(())
        } else {
            Err(MailError::Offline(format!(
                "IMAP 服务器拒绝连接：{}",
                String::from_utf8_lossy(&line).trim()
            )))
        }
    }

    fn login(&mut self, email: &str, secret: &str) -> Result<(), MailError> {
        let response = self.command(&format!(
            "LOGIN \"{}\" \"{}\"",
            quote(email),
            quote(secret)
        ))?;
        let text = String::from_utf8_lossy(&response);
        let tagged = text.lines().rev().find(|line| line.starts_with('a')).unwrap_or("");
        if tagged.contains(" OK") {
            Ok(())
        } else if tagged.contains("NO") || tagged.contains("BAD") {
            Err(MailError::AuthFailed)
        } else {
            Err(MailError::Offline("邮箱登录失败".into()))
        }
    }

    fn examine(&mut self, mailbox: &str) -> Result<u32, MailError> {
        let response = self.command(&format!("EXAMINE \"{}\"", quote(mailbox)))?;
        let text = String::from_utf8_lossy(&response);
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("* ") {
                if let Some(value) = rest.strip_suffix(" EXISTS") {
                    return value
                        .parse()
                        .map_err(|_| MailError::ReadFailed("无法解析邮件数量".into()));
                }
            }
        }
        Ok(0)
    }

    fn command(&mut self, command: &str) -> Result<Vec<u8>, MailError> {
        self.tag += 1;
        let tag = format!("a{}", self.tag);
        let line = format!("{tag} {command}\r\n");
        self.reader
            .get_mut()
            .write_all(line.as_bytes())
            .and_then(|()| self.reader.get_mut().flush())
            .map_err(|error| MailError::Offline(error.to_string()))?;
        self.read_response(&tag)
    }

    fn read_response(&mut self, tag: &str) -> Result<Vec<u8>, MailError> {
        let mut raw = Vec::new();
        loop {
            let mut line = Vec::new();
            let read = self
                .reader
                .read_until(b'\n', &mut line)
                .map_err(|error| MailError::Offline(error.to_string()))?;
            if read == 0 {
                return Err(MailError::Offline("IMAP 连接已断开".into()));
            }
            raw.extend_from_slice(&line);
            let text = String::from_utf8_lossy(&line);
            if text.starts_with(&format!("{tag} ")) {
                break;
            }
            if let Some(length) = literal_length(&line) {
                let mut buffer = vec![0u8; length];
                self.reader
                    .read_exact(&mut buffer)
                    .map_err(|error| MailError::Offline(error.to_string()))?;
                raw.extend_from_slice(&buffer);
            }
        }
        Ok(raw)
    }
}

fn split_server(server: &str) -> (&str, u16) {
    match server.split_once(':') {
        Some((name, port)) => (name, port.parse().unwrap_or(993)),
        None => (server, 993),
    }
}

fn quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn literal_length(line: &[u8]) -> Option<usize> {
    let text = String::from_utf8_lossy(line);
    let end = text.rfind('}')?;
    let start = text[..end].rfind('{')?;
    text[start + 1..end].parse().ok()
}

pub fn fetch_headers(
    account: &Account,
    secret: &str,
    limit: usize,
) -> Result<Vec<Header>, MailError> {
    let mut session = Session::connect(account, secret)?;
    let exists = session.examine("INBOX")?;
    if exists == 0 {
        return Ok(Vec::new());
    }
    let start = exists.saturating_sub(limit as u32).saturating_add(1);
    let response = session.command(&format!(
        "FETCH {start}:{exists} (UID FLAGS INTERNALDATE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE)])"
    ))?;
    let mut headers = parse_headers(&response);
    headers.sort_by_key(|header| std::cmp::Reverse(header.id.parse::<u32>().unwrap_or(0)));
    headers.truncate(limit);
    Ok(headers)
}

pub fn fetch_body(account: &Account, secret: &str, id: &str) -> Result<String, MailError> {
    let uid: u32 = id
        .parse()
        .map_err(|_| MailError::ReadFailed("邮件 UID 无效".into()))?;
    let mut session = Session::connect(account, secret)?;
    session.examine("INBOX")?;
    let response = session.command(&format!("UID FETCH {uid} (UID BODY.PEEK[1])"))?;
    let literal = parse_body_literal(&response)
        .ok_or_else(|| MailError::ReadFailed("该邮件没有可读取的文本正文".into()))?;
    Ok(String::from_utf8_lossy(&literal).into_owned())
}

fn parse_headers(raw: &[u8]) -> Vec<Header> {
    let text = format!("\r\n{}", String::from_utf8_lossy(raw));
    let mut headers = Vec::new();
    for block in text.split("\r\n* ").skip(1) {
        let Some(uid) = field_number(block, "UID ") else {
            continue;
        };
        let unread = !block.contains("\\Seen");
        let date = field_quoted(block, "INTERNALDATE ").unwrap_or_default();
        let Some(literal) = header_literal(block) else {
            continue;
        };
        let parsed = parse_header_fields(&literal);
        headers.push(Header {
            id: uid.to_string(),
            from: decode_words(parsed.0),
            subject: decode_words(parsed.1),
            date: parsed.2.or(Some(date)).unwrap_or_default(),
            unread,
        });
    }
    headers
}

fn field_number(block: &str, key: &str) -> Option<u32> {
    let start = block.find(key)? + key.len();
    block[start..]
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn field_quoted(block: &str, key: &str) -> Option<String> {
    let start = block.find(key)? + key.len();
    let rest = block[start..].trim_start();
    let quote = rest.strip_prefix('"')?;
    let end = quote.find('"')?;
    Some(quote[..end].to_string())
}

fn header_literal(block: &str) -> Option<String> {
    let marker = block.find("BODY[HEADER.FIELDS")?;
    let rest = &block[marker..];
    let open = rest.find('{')?;
    let close = rest[open..].find('}')? + open;
    let length: usize = rest[open + 1..close].parse().ok()?;
    let after = rest[close + 1..].strip_prefix("\r\n")?;
    let bytes = after.as_bytes();
    if bytes.len() < length {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes[..length]).into_owned())
}

fn parse_body_literal(raw: &[u8]) -> Option<Vec<u8>> {
    let marker = raw.windows(7).position(|window| window == b"BODY[1]")?;
    let rest = &raw[marker..];
    let open = rest.iter().position(|byte| *byte == b'{')?;
    let close = rest[open..].iter().position(|byte| *byte == b'}')? + open;
    let length: usize = std::str::from_utf8(&rest[open + 1..close]).ok()?.parse().ok()?;
    let start = close + 1;
    let start = if rest.get(start..start + 2) == Some(b"\r\n") { start + 2 } else { start };
    rest.get(start..start + length).map(ToOwned::to_owned)
}

fn parse_header_fields(header: &str) -> (String, String, Option<String>) {
    let mut from = String::new();
    let mut subject = String::new();
    let mut date = None;
    let mut current = String::new();
    for line in header.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            current.push(' ');
            current.push_str(line.trim());
            continue;
        }
        if !current.is_empty() {
            assign_header(&current, &mut from, &mut subject, &mut date);
        }
        current = line.to_string();
    }
    if !current.is_empty() {
        assign_header(&current, &mut from, &mut subject, &mut date);
    }
    (from, subject, date)
}

fn assign_header(line: &str, from: &mut String, subject: &mut String, date: &mut Option<String>) {
    let Some((key, value)) = line.split_once(':') else {
        return;
    };
    let value = value.trim().to_string();
    match key.trim().to_ascii_lowercase().as_str() {
        "from" => *from = value,
        "subject" => *subject = value,
        "date" => *date = Some(value),
        _ => {}
    }
}

fn decode_words(value: String) -> String {
    let mut out = String::new();
    let mut rest = value.as_str();
    while let Some(start) = rest.find("=?") {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find("?=") else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let token = &rest[start..start + end + 2];
        out.push_str(&decode_word(token).unwrap_or_else(|| token.to_string()));
        rest = &rest[start + end + 2..];
    }
    out.push_str(rest);
    out
}

fn decode_word(token: &str) -> Option<String> {
    let body = token.strip_prefix("=?")?.strip_suffix("?=")?;
    let mut parts = body.splitn(3, '?');
    let _charset = parts.next()?;
    let encoding = parts.next()?.to_ascii_uppercase();
    let data = parts.next()?;
    let bytes = match encoding.as_str() {
        "B" => STANDARD.decode(data).ok()?,
        "Q" => decode_q(data),
        _ => return None,
    };
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn decode_q(data: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let bytes = data.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'_' => {
                out.push(b' ');
                index += 1;
            }
            b'=' if index + 2 < bytes.len() => {
                if let Ok(value) = u8::from_str_radix(&data[index + 1..index + 3], 16) {
                    out.push(value);
                    index += 3;
                } else {
                    out.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "network: verifies TLS and the authentication failure path"]
    fn live_tls_reaches_an_imap_server_and_rejects_bad_credentials() {
        let account = Account {
            email: "nsh-test@example.invalid".to_string(),
            host: "imap.gmail.com".to_string(),
            enabled: true,
        };
        match fetch_headers(&account, "invalid", 1) {
            Err(MailError::AuthFailed) => {}
            Err(MailError::Offline(error)) => {
                println!("IMAP live probe skipped: {error}");
            }
            Err(MailError::Timeout) => {
                println!("IMAP live probe timed out");
            }
            other => panic!("unexpected IMAP result: {other:?}"),
        }
    }
    #[test]
    fn quoted_credentials_and_literal_lengths() {
        assert_eq!(quote("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(literal_length(b"a1 FETCH {12}\r\n"), Some(12));
    }

    #[test]
    fn headers_parse_uid_flags_internal_date_and_encoded_words() {
        let header = "From: =?UTF-8?B?5Zui6Zif?=\r\nSubject: =?UTF-8?Q?hello_=E4=B8=96=E7=95=8C?=\r\n\r\n";
        let raw = format!(
            "* 1 FETCH (UID 42 FLAGS (\\Seen) INTERNALDATE \"24-Nov-2026 10:00:00 +0800\" BODY[HEADER.FIELDS (FROM SUBJECT DATE)] {{{}}}\r\n{})\r\n",
            header.len(),
            header
        );
        let headers = parse_headers(raw.as_bytes());
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].id, "42");
        assert_eq!(headers[0].from, "团队");
        assert_eq!(headers[0].subject, "hello 世界");
        assert!(headers[0].date.contains("24-Nov-2026"));
        assert!(!headers[0].unread);
    }

    #[test]
    fn body_literal_reads_only_the_requested_part() {
        let raw = b"* 1 FETCH (UID 42 BODY[1] {11}\r\nhello world)\r\n";
        assert_eq!(parse_body_literal(raw).unwrap(), b"hello world");
    }

    #[test]
    fn no_literal_means_no_text_body() {
        assert!(parse_body_literal(b"* 1 FETCH (UID 42 BODY[1] NIL)\r\n").is_none());
    }
}
