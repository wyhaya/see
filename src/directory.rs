use crate::util::bytes_to_size;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use time::Timespec;
use tokio::fs;
use tokio::io::Result;
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
            font-family: "pingfang sc", "microsoft yahei", "Helvetica Neue";
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
    path: &PathBuf,
    title: &str,
    time: &Option<String>,
    size: bool,
) -> Result<String> {
    let mut dir = fs::read_dir(path).await?;
    let mut content = String::new();

    while let Some(entry) = dir.next().await {
        let entry = entry?.path();

        match entry.file_name() {
            Some(d) => match d.to_str() {
                Some(name) => {
                    if entry.is_dir() {
                        content.push_str(&format!("<a href=\"{}/\">{}/</a>", name, name));
                    } else {
                        content.push_str(&format!("<a href=\"{}\">{}</a>", name, name));
                    }
                }
                None => continue,
            },
            None => continue,
        };

        if time.is_some() || size {
            let meta = fs::metadata(&entry).await?;

            if let Some(format) = &time {
                let dur = meta.modified()?.duration_since(UNIX_EPOCH).unwrap();
                let spec = Timespec::new(dur.as_secs() as i64, dur.subsec_nanos() as i32);

                content.push_str(&format!(
                    "<time>{}</time>",
                    time::at(spec).strftime(format).unwrap()
                ));
            }

            if size {
                if entry.is_file() {
                    content.push_str(&format!("<span>{}</span>", bytes_to_size(meta.len())));
                } else {
                    content.push_str("<span></span>");
                }
            }
        }
    }

    let (mut columns, mut column) = ("auto auto 1fr", "1 / 4");

    if time.is_none() && !size {
        columns = "auto";
        column = "1 / 2";
    } else if (time.is_none() && size) || (time.is_some() && !size) {
        columns = "auto 1fr";
        column = "1 / 3";
    }

    let template = TEMPLATE
        .replacen("{title}", title, 2)
        .replacen("{columns}", columns, 1)
        .replacen("{column}", column, 1)
        .replacen("{content}", &content, 1);

    Ok(template)
}
