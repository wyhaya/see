// from: https://developer.cdn.mozilla.net/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Complete_list_of_MIME_types

pub const TEXT_HTML: &str = "text/html";
pub const TEXT_PLAIN: &str = "text/plain";

pub fn from_extension(ext: &str) -> &'static str {
    match ext {
        "aac" => "audio/aac",
        "mp3" => "audio/mpeg",
        "oga" => "audio/ogg",
        "wav" => "audio/wav",
        "weba" => "audio/webm",

        "abw" => "application/x-abiword",
        "arc" => "application/x-freearc",
        "azw" => "application/vnd.amazon.ebook",
        "bz" => "application/x-bzip",
        "bz2" => "application/x-bzip2",
        "csh" => "application/x-csh",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "eot" => "application/vnd.ms-fontobject",
        "epub" => "application/epub+zip",
        "jar" => "application/java-archive",
        "json" => "application/json",
        "mpkg" => "application/vnd.apple.installer+xml",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ogx" => "application/ogg",
        "pdf" => "application/pdf",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "rar" => "application/x-rar-compressed",
        "rtf" => "application/rtf",
        "sh" => "application/x-sh",
        "swf" => "application/x-shockwave-flash",
        "tar" => "application/x-tar",
        "vsd" => "application/vnd.visio",
        "xhtml" => "application/xhtml+xml",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xul" => "application/vnd.mozilla.xul+xml",
        "zip" => "application/zip",
        "7z" => "application/x-7z-compressed",

        "bmp" => "image/bmp",
        "gif" => "image/gif",
        "ico" => "image/vnd.microsoft.icon",
        "jpeg" | "jpg" => "image/jpeg",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "tif" => "image/tiff",
        "tiff" => "image/tiff",
        "webp" => "image/webp",

        "css" => "text/css",
        "csv" => "text/csv",
        "htm" | "html" => TEXT_HTML,
        "ics" => "text/calendar",
        "js" | "mjs" => "text/javascript",
        "txt" => TEXT_PLAIN,
        "xml" => "text/xml",

        "otf" => "font/otf",
        "ttf" => "font/ttf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",

        "avi" => "video/x-msvideo",
        "mpeg" => "video/mpeg",
        "ogv" => "video/ogg",
        "webm" => "video/webm",
        "3gp" => "video/3gpp",
        "3g2" => "video/3gpp2",

        _ => "application/octet-stream",
    }
}