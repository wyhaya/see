use std::net::SocketAddr;
use std::time::Duration;

pub fn dedup<T: Eq>(vec: Vec<T>) -> Vec<T> {
    let mut new = vec![];
    for item in vec {
        if !new.contains(&item) {
            new.push(item);
        }
    }
    new
}

#[derive(Debug)]
pub enum ParseDurationError {
    NoNumber,
    NoUnit,
    ErrorNumber,
    ErrorUnit,
    Zero,
}

impl ParseDurationError {
    pub fn description(&self) -> &str {
        match self {
            ParseDurationError::NoNumber => "no number",
            ParseDurationError::NoUnit => "no unit",
            ParseDurationError::ErrorNumber => "error number",
            ParseDurationError::ErrorUnit => "error unit",
            ParseDurationError::Zero => "zero",
        }
    }
}

// Parse time format into Duration
// format: 1d 1.2h 5s ...
pub fn try_parse_duration(text: &str) -> Result<Duration, ParseDurationError> {
    let numbers = "0123456789.".chars().collect::<Vec<char>>();
    let i = text
        .chars()
        .position(|ch| !numbers.contains(&ch))
        .ok_or_else(|| ParseDurationError::NoUnit)?;

    let time = &text[..i];
    let unit = &text[i..];

    if time.is_empty() {
        return Err(ParseDurationError::NoNumber);
    }
    let n = time
        .parse::<f64>()
        .map_err(|_| ParseDurationError::ErrorNumber)?;
    let ms = match unit {
        "d" => Ok(24_f64 * 60_f64 * 60_f64 * 1000_f64 * n),
        "h" => Ok(60_f64 * 60_f64 * 1000_f64 * n),
        "m" => Ok(60_f64 * 1000_f64 * n),
        "s" => Ok(1000_f64 * n),
        "ms" => Ok(n),
        _ => Err(ParseDurationError::ErrorUnit),
    }? as u64;

    if ms == 0 {
        Err(ParseDurationError::Zero)
    } else {
        Ok(Duration::from_millis(ms))
    }
}

pub fn try_to_socket_addr(text: &str) -> Result<SocketAddr, ()> {
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Ok(addr);
    }
    if let Ok(port) = text.parse::<i64>() {
        if let Ok(addr) = format!("0.0.0.0:{}", port).parse::<SocketAddr>() {
            return Ok(addr);
        }
    }

    Err(())
}

pub fn bytes_to_size(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    if bytes < 1024 {
        return format!("{}.00 B", bytes);
    }
    let bytes = bytes as f64;
    let i = (bytes.ln() / 1024_f64.ln()) as i32;
    format!("{:.2} {}", bytes / 1024_f64.powi(i), UNITS[i as usize])
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn to_size() {
        assert_eq!(bytes_to_size(0), "0.00 B");
        assert_eq!(bytes_to_size(1), "1.00 B");
        assert_eq!(bytes_to_size(1023), "1023.00 B");
        assert_eq!(bytes_to_size(1024), "1.00 KB");
        assert_eq!(bytes_to_size(1 * 1024 * 1024), "1.00 MB");
        assert_eq!(bytes_to_size(1 * 1024 * 1024 * 1024 * 1024), "1.00 TB");
        assert_eq!(bytes_to_size(u64::max_value()), "16.00 EB");
    }
}
