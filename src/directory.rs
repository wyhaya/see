use crate::util::bytes_to_size;
use futures_util::future::try_join_all;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use time::Timespec;
use tokio::fs::{self, DirEntry};
use tokio::io::{self, Error, ErrorKind, Result};
use tokio::stream::StreamExt;

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

pub async fn render_dir_html(
    dir: &PathBuf,
    title: &str,
    time: &Option<String>,
    size: bool,
) -> io::Result<String> {
    let mut dir = fs::read_dir(dir).await?;
    let mut content = String::new();
    let mut fus = vec![];

    while let Some(entry) = dir.next().await {
        let entry = entry?;
        fus.push(get_entry_content(entry, &time, size));
    }

    try_join_all(fus).await?.iter().for_each(|s| {
        content.push_str(s);
    });

    let (columns, column) = match (time, size) {
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

async fn get_entry_content(entry: DirEntry, time: &Option<String>, size: bool) -> Result<String> {
    let mut content = String::new();

    let os_name = entry.file_name();

    let name = os_name
        .to_str()
        .ok_or_else(|| Error::new(ErrorKind::Other, ""))?;

    let meta = entry.metadata().await?;
    let is_file = meta.is_file();

    if is_file {
        content.push_str(&format!("<a href=\"{}\">{}</a>", name, name));
    } else {
        content.push_str(&format!("<a href=\"{}/\">{}/</a>", name, name));
    }

    if let Some(format) = &time {
        let dur = meta
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::new(ErrorKind::Other, ""))?;
        let spec = Timespec::new(dur.as_secs() as i64, dur.subsec_nanos() as i32);

        content.push_str(&format!(
            "<time>{}</time>",
            time::at(spec).strftime(format).unwrap()
        ));
    }

    if size {
        if is_file {
            let size = bytes_to_size(meta.len());
            content.push_str(&format!("<span>{}</span>", size));
        } else {
            content.push_str("<span></span>");
        }
    }

    Ok(content)
}
