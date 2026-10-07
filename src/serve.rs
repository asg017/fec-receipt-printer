//! The debug viewer: a local web page that renders a filing's receipt at true
//! scale (1 CSS px = 1 printer dot) without printing it, with a column grid,
//! the exact ESC/POS bytes, and a Print button for when the printer is
//! plugged in.
//!
//! Server-rendered, no JS framework: `GET /?src=…&full=1` is the whole page.

use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use tiny_http::{Header, Method, Request, Response, Server};

use crate::{
    escpos, forms,
    preview::{self, escape as esc},
    source, usb,
};

pub fn run(host: &str, port: u16, samples: Option<PathBuf>) -> anyhow::Result<()> {
    let server = Server::http((host, port)).map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("debug viewer on http://{host}:{port}/");
    for request in server.incoming_requests() {
        if let Err(e) = handle(request, samples.as_deref()) {
            eprintln!("request failed: {e}");
        }
    }
    Ok(())
}

struct Query {
    src: String,
    full: bool,
    msg: Option<String>,
}

impl Query {
    fn parse(url: &str) -> Self {
        let mut q = Query {
            src: String::new(),
            full: false,
            msg: None,
        };
        let query = url.split_once('?').map(|(_, q)| q).unwrap_or("");
        for pair in query.split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let v = decode(v);
            match k {
                "src" => q.src = v,
                "full" => q.full = !v.is_empty() && v != "0",
                "msg" => q.msg = Some(v),
                _ => {}
            }
        }
        q
    }

    fn to_url(&self, path: &str) -> String {
        format!(
            "{path}?src={}{}",
            encode(&self.src),
            if self.full { "&full=1" } else { "" }
        )
    }
}

fn handle(request: Request, samples: Option<&Path>) -> std::io::Result<()> {
    let url = request.url().to_owned();
    let path = url.split('?').next().unwrap_or("/");
    let q = Query::parse(&url);
    match (request.method(), path) {
        (Method::Get, "/") => {
            let page = page(&q, samples);
            request.respond(
                Response::from_string(page).with_header(header("text/html; charset=utf-8")),
            )
        }
        (Method::Get, "/receipt.bin") => match render(&q) {
            Ok((_, bytes)) => request.respond(Response::new(
                200.into(),
                vec![header("application/octet-stream")],
                Cursor::new(bytes.clone()),
                Some(bytes.len()),
                None,
            )),
            Err(e) => request.respond(Response::from_string(e.to_string()).with_status_code(500)),
        },
        (Method::Post, "/print") => {
            let msg = match render(&q).and_then(|(_, bytes)| usb::print(&bytes)) {
                Ok(()) => "Printed.".to_owned(),
                Err(e) => format!("Print failed: {e:#}"),
            };
            let back = format!("{}&msg={}", q.to_url("/"), encode(&msg));
            request.respond(
                Response::empty(303).with_header(Header::from_bytes("Location", back).unwrap()),
            )
        }
        _ => request.respond(Response::from_string("not found").with_status_code(404)),
    }
}

fn render(q: &Query) -> anyhow::Result<(crate::receipt::Receipt, Vec<u8>)> {
    let mut filing = source::open(&q.src)?;
    let receipt = forms::render(&mut filing, forms::Opts { full: q.full });
    let bytes = escpos::encode(&receipt);
    Ok((receipt, bytes))
}

fn page(q: &Query, samples: Option<&Path>) -> String {
    let printer = usb::connected();
    let mut main = String::new();
    let mut side = String::new();

    if let Some(msg) = &q.msg {
        side.push_str(&format!(r#"<p class="msg">{}</p>"#, esc(msg)));
    }

    if q.src.is_empty() {
        main.push_str(r#"<p class="empty">Enter a filing ID, URL or path, or pick a sample.</p>"#);
    } else {
        match render(q) {
            Ok((receipt, bytes)) => {
                main.push_str(&preview::html(&receipt));
                side.push_str(&format!(
                    r#"<dl class="stats"><dt>Paper</dt><dd>~{:.0} mm</dd><dt>Lines</dt><dd>{}</dd><dt>Bytes</dt><dd>{}</dd></dl>"#,
                    preview::length_mm(&receipt),
                    receipt.line_count(),
                    bytes.len()
                ));
                if !receipt.warnings.is_empty() {
                    side.push_str(r#"<div class="warn"><b>Warnings</b><ul>"#);
                    for w in &receipt.warnings {
                        side.push_str(&format!("<li>{}</li>", esc(w)));
                    }
                    side.push_str("</ul></div>");
                }
                side.push_str(&format!(
                    r#"<form method="post" action="{}"><button {}>Print</button> <span class="dot {}"></span> {}</form>
                    <p><a href="{}">Download .bin</a></p>"#,
                    esc(&q.to_url("/print")),
                    if printer { "" } else { "disabled" },
                    if printer { "on" } else { "off" },
                    if printer { "printer connected" } else { "printer not connected" },
                    esc(&q.to_url("/receipt.bin")),
                ));
                main.push_str(&format!(
                    "<details><summary>ESC/POS bytes ({} bytes)</summary><pre class=\"hex\">{}</pre></details>",
                    bytes.len(),
                    esc(&escpos::hexdump(&bytes))
                ));
            }
            Err(e) => main.push_str(&format!(
                r#"<pre class="err">{}</pre>"#,
                esc(&format!("{e:#}"))
            )),
        }
    }

    let mut sample_links = String::new();
    if let Some(dir) = samples {
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "fec"))
            .collect();
        files.sort();
        for f in files {
            let name = f.file_stem().unwrap_or_default().to_string_lossy();
            let link = Query {
                src: f.display().to_string(),
                full: q.full,
                msg: None,
            }
            .to_url("/");
            let current = if f.display().to_string() == q.src {
                r#" class="cur""#
            } else {
                ""
            };
            sample_links.push_str(&format!(
                r#"<a href="{}"{current}>{}</a>"#,
                esc(&link),
                esc(&name)
            ));
        }
    }

    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Receipt Viewer</title><style>{css}{paper}</style></head>
<body><aside>
<h1>FEC receipt</h1>
<form method="get" action="/">
  <input name="src" value="{src}" placeholder="1926068, URL, or path" autofocus>
  <label><input type="checkbox" name="full" value="1" {full}> full detail</label>
  <label><input type="checkbox" id="grid"> column grid</label>
  <button>Render</button>
</form>
{side}
<nav class="samples">{sample_links}</nav>
</aside>
<main>{main}</main>
<script>
const g = document.getElementById('grid');
g.checked = localStorage.getItem('grid') === '1';
document.body.classList.toggle('grid', g.checked);
g.onchange = () => {{ document.body.classList.toggle('grid', g.checked); try {{ localStorage.setItem('grid', g.checked ? '1' : '0') }} catch {{}} }};
</script>
</body></html>"#,
        css = PAGE_CSS,
        paper = preview::PAPER_CSS,
        src = esc(&q.src),
        full = if q.full { "checked" } else { "" },
    )
}

const PAGE_CSS: &str = r#"
:root { --bg: #e7e5e0; --fg: #222; --muted: #6b6b6b; --line: #cfccc4; --panel: #f4f3ef; --accent: #2563eb; }
@media (prefers-color-scheme: dark) { :root { --bg: #1f1f22; --fg: #e8e8e8; --muted: #9a9a9a; --line: #3a3a3f; --panel: #28282c; --accent: #7aa2ff; } }
* { box-sizing: border-box; }
body { margin: 0; display: flex; min-height: 100vh; background: var(--bg); color: var(--fg); font: 14px/1.4 system-ui, sans-serif; }
aside { width: 280px; flex: none; padding: 16px; border-right: 1px solid var(--line); background: var(--panel); overflow-y: auto; max-height: 100vh; position: sticky; top: 0; }
h1 { font-size: 15px; margin: 0 0 12px; }
form { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; }
form[method=post] { flex-direction: row; align-items: center; }
input[name=src] { padding: 6px 8px; font: inherit; border: 1px solid var(--line); border-radius: 6px; background: var(--bg); color: var(--fg); }
button { padding: 6px 12px; font: inherit; border-radius: 6px; border: 1px solid var(--line); background: var(--accent); color: #fff; cursor: pointer; }
button[disabled] { background: var(--line); color: var(--muted); cursor: not-allowed; }
a { color: var(--accent); }
.dot { width: 8px; height: 8px; border-radius: 50%; display: inline-block; }
.dot.on { background: #16a34a; } .dot.off { background: #9ca3af; }
.stats { display: grid; grid-template-columns: auto 1fr; gap: 2px 12px; margin: 0 0 12px; }
.stats dt { color: var(--muted); } .stats dd { margin: 0; font-variant-numeric: tabular-nums; }
.warn { background: #fef3c7; color: #78350f; padding: 8px; border-radius: 6px; margin-bottom: 12px; }
.warn ul { margin: 4px 0 0; padding-left: 18px; }
.msg { background: var(--bg); padding: 8px; border-radius: 6px; }
.samples { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 16px; border-top: 1px solid var(--line); padding-top: 12px; }
.samples a { font: 12px ui-monospace, monospace; padding: 2px 6px; border-radius: 4px; color: var(--fg); text-decoration: none; border: 1px solid var(--line); }
.samples a.cur, .samples a:hover { border-color: var(--accent); color: var(--accent); }
main { flex: 1; padding: 32px 16px; display: flex; flex-direction: column; align-items: center; gap: 24px; min-width: 0; }
details { width: 100%; max-width: 720px; }
.hex { font: 12px/1.4 ui-monospace, monospace; overflow-x: auto; background: var(--panel); padding: 12px; border-radius: 6px; }
.err { color: #dc2626; white-space: pre-wrap; max-width: 600px; }
.empty { color: var(--muted); }
@media (max-width: 720px) { body { flex-direction: column; } aside { width: auto; position: static; max-height: none; border-right: 0; border-bottom: 1px solid var(--line); } }
"#;

fn header(content_type: &str) -> Header {
    Header::from_bytes("Content-Type", content_type).unwrap()
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(b) => {
                        out.push(b);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
