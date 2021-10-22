use futures_util::future::try_join_all;
use lazy_static::lazy_static;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};
use time::{OffsetDateTime, UtcOffset};
use tokio::fs::{self, DirEntry};

// HTML directory template
const TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Index of {title}</title>
    <style>
        body {
            font-family: "Segoe UI", Segoe,Tahoma,Arial, Verdana, sans-serif;
            padding: 0 16px 0;
            margin: 0;
        }
        h1 {
            font-weight: normal;
            word-wrap: break-word;
        }
        main {
            display: grid;
            grid-template-columns: {columns};
        }
        a:first-child {
            grid-column: {column};
        }
        a, time, span {
            height: 28px;
            line-height: 28px;
            text-overflow: ellipsis;
            overflow: hidden;
            white-space: nowrap;
        }
        a {
            color: #2a7ae2;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
        a:active, a:visited {
            color: #1756a9;
        }
        time, span {
            padding-left: 16px;
        }
        @media (prefers-color-scheme: dark) {
            body {
                background-color: #1e2022;
                color: #d5d5d5;
            }
        }
    </style>
</head>
<body>
    <h1>Index of {title}</h1>
    <main>
        <a href="../">../</a>
        {content}
    </main>
</body>
</html>
"#;

#[derive(Debug, Clone)]
pub struct Directory {
    pub time: Option<String>,
    pub size: bool,
}

impl Directory {
    pub async fn render(&self, dir: &Path, title: &str) -> Result<String, ()> {
        let mut dir = fs::read_dir(dir).await.map_err(|_| ())?;
        let mut fus = vec![];

        loop {
            let entry = match dir.next_entry().await {
                Ok(opt) => match opt {
                    Some(entry) => entry,
                    None => break,
                },
                Err(_) => return Err(()),
            };
            if let Some(name) = entry.file_name().to_str() {
                if !name.starts_with('.') {
                    fus.push(Self::render_row(entry, &self.time, self.size));
                }
            } else {
                return Err(());
            }
        }

        let content = try_join_all(fus).await?.join("");

        let (columns, column) = match (&self.time, self.size) {
            // Show only the name
            (None, false) => ("auto", "1 / 2"),
            // Show name, time, size
            (Some(_), true) => ("auto auto 1fr", "1 / 4"),
            // Show name time or name size
            _ => ("auto 1fr", "1 / 3"),
        };

        let template = TEMPLATE
            .replacen("{title}", title, 2)
            .replacen("{columns}", columns, 1)
            .replacen("{column}", column, 1)
            .replacen("{content}", &content, 1);

        Ok(template)
    }

    async fn render_row(entry: DirEntry, time: &Option<String>, size: bool) -> Result<String, ()> {
        let meta = entry.metadata().await.map_err(|_| ())?;
        let name = entry.file_name();
        let name = name.to_str().unwrap();
        let mut content = String::new();

        if meta.is_file() {
            content.push_str(&format!("<a href=\"{}\">{}</a>", name, name));
        } else {
            content.push_str(&format!("<a href=\"{}/\">{}/</a>", name, name));
        }

        if let Some(format) = &time {
            let dur = meta
                .modified()
                .map_err(|_| ())?
                .duration_since(UNIX_EPOCH)
                .map_err(|_| ())?;

            let s = format!("<time>{}</time>", format_datetime(dur, format));
            content.push_str(&s);
        }

        if size {
            if meta.is_file() {
                content.push_str(&format!("<span>{}</span>", format_size(meta.len())));
            } else {
                content.push_str("<span></span>");
            }
        }

        Ok(content)
    }
}

fn format_datetime(dur: Duration, format: &str) -> String {
    lazy_static! {
        static ref UTC_OFFSET: UtcOffset = UtcOffset::try_current_local_offset().unwrap();
    }
    let datetime = OffsetDateTime::from_unix_timestamp(dur.as_secs() as i64).to_offset(*UTC_OFFSET);
    datetime.format(format)
}

fn format_size(n: u64) -> String {
    const UNITS: [char; 6] = ['K', 'M', 'G', 'T', 'P', 'E'];
    if n < 1024 {
        return format!("{} B", n);
    }
    let bytes = n as f64;
    let i = (bytes.ln() / 1024_f64.ln()) as i32;
    format!(
        "{:.2} {}B",
        bytes / 1024_f64.powi(i),
        UNITS[(i - 1) as usize]
    )
}

#[test]
fn test_format_size() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(1), "1 B");
    assert_eq!(format_size(1023), "1023 B");
    assert_eq!(format_size(1024), "1.00 KB");
    assert_eq!(format_size(1 * 1024 * 1024), "1.00 MB");
    assert_eq!(format_size(1 * 1024 * 1024 * 1024 * 1024), "1.00 TB");
    assert_eq!(format_size(u64::max_value()), "16.00 EB");
}
