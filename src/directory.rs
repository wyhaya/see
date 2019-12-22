use crate::config::Directory;
use chrono::{DateTime, Local};
use futures::StreamExt;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::Result;

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
            padding: 0 24px 0;
        }
        h1 {
            font-weight: normal;
            word-wrap: break-word;
        }
        main{
            display: grid;
            grid-template-columns: {columns};
        }
        a:first-child{
            grid-column: {column};
        }
        a, time, span{
            line-height: 20px;
            word-wrap: break-word;
            margin-top: 6px;
        }
        time, span{
            padding-left: 20px;
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

pub async fn render_dir_html(path: &PathBuf, title: &str, option: &Directory) -> Result<String> {
    let mut dir = fs::read_dir(path).await?;

    let (mut columns, mut column) = ("auto auto 1fr", "1 / 4");

    if !option.time && !option.size {
        columns = "auto";
        column = "1 / 2";
    } else if (!option.time && option.size) || (option.time && !option.size) {
        columns = "auto 1fr";
        column = "1 / 3";
    }

    let template = TEMPLATE
        .replacen("{title}", title, 2)
        .replacen("{columns}", columns, 1)
        .replacen("{column}", column, 1);

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

        if option.size || option.time {
            let meta = fs::metadata(&entry).await?;

            if option.time {
                let datetime: DateTime<Local> = DateTime::from(meta.modified()?);
                content.push_str(&format!(
                    "<time>{}</time>",
                    datetime.format("%Y-%m-%d %H:%M")
                ));
            }

            if option.size {
                if entry.is_file() {
                    content.push_str(&format!(
                        "<span>{}</span>",
                        bytes_to_size(meta.len() as f64)
                    ));
                } else {
                    content.push_str("<span></span>");
                }
            }
        }
    }

    Ok(template.replacen("{content}", &content, 1))
}

fn bytes_to_size(bytes: f64) -> String {
    let unit = 1024_f64;
    let sizes = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    if bytes <= 1_f64 {
        return format!("{:.2} B", bytes);
    }
    let i = (bytes.ln() / unit.ln()) as i32;
    format!("{:.2} {}", bytes / unit.powi(i), sizes[i as usize])
}
