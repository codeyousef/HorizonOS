//! Only this sanitized representation may become journal evidence.
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Message {
    message: String,
    redacted: bool,
}
impl Message {
    pub fn text(&self) -> &str { &self.message }
    pub fn redacted(&self) -> bool { self.redacted }
}

fn hidden(reason: &str) -> Message {
    Message { message: format!("[REDACTED {reason}]"), redacted: true }
}

fn sensitive(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    // Whole-line removal also covers quoted/escaped values, multiline argv,
    // authorization schemes and connection-string query parameters. Never
    // preserve the remainder of an unfamiliar credential syntax.
    let labels = ["password", "passwd", "passphrase", "bearer", "authorization", "cookie",
        "api_key", "api-key", "apikey", "access_token", "access-token", "refresh_token", "refresh-token",
        "secret", "token", "credential", "connection string", "connection_string", "private key", "private_key",
        "environment", "execstart", "command line", "argv", "dsn"];
    if labels.iter().any(|word| lower.contains(word)) || lower.contains("://") { return true; }
    if line.split_whitespace().any(|raw| { let word=raw.trim_start_matches(|c:char|!c.is_ascii_alphanumeric() && c!='-');
        word.starts_with("sk-") && word.len() >= 16
        || word.starts_with("eyJ") && word.matches('.').count() >= 2
        || word.starts_with("AKIA") && word.len() >= 20
        || word.starts_with('-') && word.as_bytes().get(1).is_some_and(u8::is_ascii_alphabetic)
        || word.starts_with("--") }) { return true; }
    // Structured environment dumps may use JSON key:value instead of '='.
    if line.split(':').any(|prefix| prefix.rsplit('"').nth(1).is_some_and(|key|
        key.len()>=2 && key.chars().any(|c|c.is_ascii_uppercase())
            && key.chars().all(|c|c.is_ascii_uppercase() || c.is_ascii_digit() || c=='_'))) { return true; }
    // Environment variables and unknown key=value arguments are sensitive by
    // default; an unfamiliar variable name is not a redaction bypass.
    line.split('=').next().filter(|_| line.contains('=')).is_some_and(|prefix| {
        prefix.trim_end().chars().next_back().is_some_and(|c| c.is_ascii_alphanumeric() || "_\"'".contains(c))
    })
}

pub fn redact(bytes: &[u8]) -> Message {
    if bytes.len() > 4096 { return hidden("OVERSIZED MESSAGE"); }
    let Ok(text) = std::str::from_utf8(bytes) else { return hidden("BINARY MESSAGE"); };
    if text.chars().any(|c| (c.is_control() && c != '\n' && c != '\t' && c != '\r')
        || matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}')) {
        return hidden("CONTROL CHARACTERS");
    }
    if text.contains("-----BEGIN") { return hidden("KEY MATERIAL"); }
    let mut redacted = false;
    let lines: Vec<_> = text.lines().map(|line| {
        if sensitive(line) { redacted = true; "[REDACTED SENSITIVE LINE]" } else { line }
    }).collect();
    let message = lines.join("\n");
    // Replacement can grow many tiny lines beyond the original input bound.
    if message.len() > 4096 { return hidden("OVERSIZED REDACTION"); }
    Message { message, redacted }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_environment_and_arguments_never_survive() {
        for text in ["Authorization: Basic ZmFrZTpmYWtl", "Bearer fake-token", "PASSWORD='fake value'",
            "{\"apiKey\":\"fake\"}", "{\"MYSTERY_VARIABLE\":\"fake\"}", "{\"token\":\"fake\"}",
            "postgres://user:fake@host/database", "MYSTERY_VARIABLE=fake",
            "tool --custom-secret fake", "tool -p fake", "{\"cookie\":\"fake=value\"}",
            "eyJhbGci.fake.signature", "sk-abcdefghijklmnop", "AKIAABCDEFGHIJKLMNOP", "\"sk-abcdefghijklmnop\""] {
            let value = redact(text.as_bytes());
            assert!(value.redacted(), "{text}");
            assert!(!value.text().contains("fake"));
        }
    }
    #[test]
    fn normal_unicode_diagnostics_survive_and_multiline_secrets_do_not() {
        let safe = "Failed to start redis.service.\nتعذر تشغيل الخدمة";
        assert_eq!(redact(safe.as_bytes()).text(), safe);
        assert!(!redact(safe.as_bytes()).redacted());
        let value = redact(b"Failed to start redis.service.\nTOKEN=fake\nTry restarting the service.");
        assert!(value.redacted()); assert!(!value.text().contains("fake"));
        assert!(value.text().contains("Failed to start")); assert!(value.text().contains("Try restarting"));
    }
    #[test]
    fn hostile_encoding_keys_controls_and_output_growth_fail_closed() {
        // Assemble only the synthetic test header at runtime; source transfer
        // correctly refuses literal private-key headers in repository files.
        let key = [b"-----BEGIN RSA ".as_slice(), b"PRIVATE KEY-----\nfake\n-----END RSA PRIVATE KEY-----"].concat();
        for input in [b"\xfffake".as_slice(), b"PASSWORD\x1b[0m=fake", key.as_slice(), b"foo\0fake"] {
            let value = redact(input); assert!(value.redacted()); assert!(!value.text().contains("fake"));
        }
        assert!(redact("pass\u{200b}word=fake".as_bytes()).redacted());
        assert!(redact(&vec![b'a'; 4097]).redacted());
        let value = redact("a=b\n".repeat(1000).as_bytes());
        assert!(value.redacted()); assert!(value.text().len() <= 4096);
    }
}
