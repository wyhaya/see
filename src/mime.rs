use hyper::header::HeaderValue;

pub fn from_extension(ext: &str) -> HeaderValue {
    let mime = mime_guess::from_ext(ext).first_or_octet_stream();
    HeaderValue::from_str(mime.as_ref()).unwrap()
}

pub fn text_html() -> HeaderValue {
    HeaderValue::from_static("text/html")
}

pub fn text_plain() -> HeaderValue {
    HeaderValue::from_static("text/plain")
}
