use serde::Serialize;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const VERSION_TEXT: &str = concat!("insane marmot matrix native ", env!("CARGO_PKG_VERSION"));

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    pub name: &'static str,
    pub short_name: &'static str,
    pub version: &'static str,
    pub extension: &'static str,
    pub commands: Vec<&'static str>,
    pub entrypoints: Vec<&'static str>,
    pub keywords: Vec<&'static str>,
    pub libraries: Vec<&'static str>,
    pub object_model: Vec<&'static str>,
}

pub fn spec() -> Spec {
    Spec {
        name: "insane marmot matrix",
        short_name: "IMM",
        version: VERSION,
        extension: ".imm",
        commands: vec!["run", "check", "fmt", "probe", "law", "pack", "spec"],
        entrypoints: vec![
            "marmot main",
            "insane marmot main",
            "howl marmot main",
            "insane howl marmot main",
        ],
        keywords: vec![
            "marmot", "insane", "dig", "let", "stash", "return", "if", "else", "for", "in",
            "while", "break", "continue", "true", "false", "null", "matrix", "burrow", "use",
            "squeak", "sniff", "panic", "try", "catch", "tunnel", "den", "hatch", "self", "init",
            "fur", "fang", "mask", "wear", "under", "web", "fetch", "grab", "howl", "wait",
            "scatter", "nest", "nap", "tick", "pack", "crate", "pelt", "probe", "law", "expect",
            "trace",
        ],
        libraries: vec![
            "core", "math", "matrix", "path", "chaser", "store", "web", "tick",
        ],
        object_model: vec![
            "den", "hatch", "self", "fur", "fang", "mask", "wear", "under",
        ],
    }
}

pub fn render_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&spec())
}
