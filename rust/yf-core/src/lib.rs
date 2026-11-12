//! YellowFruit core: data model, file loading and the stats engine, ported from the TypeScript app
//! (`src/renderer/DataModel/`). The goal of this crate is output parity with the TS code on real
//! tournament files; see `rust/README.md` for how that is checked.

pub mod js;
pub mod jsval;
pub mod legacy;
pub mod model;
pub mod sqbs;
pub mod stats;
pub mod tournament;
pub mod validation;
pub mod writer;
pub mod yft;

pub use tournament::Tournament;

/// Which loader handled a file
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Legacy,
    Yft,
}

impl Format {
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::Legacy => "legacy",
            Format::Yft => "yft",
        }
    }
}

pub struct Loaded {
    pub tournament: Tournament,
    pub warnings: Vec<String>,
}

/// Open a .yft file the way TournamentManager.openYftFile does
pub fn open_file(contents: &str, cur_version: &str) -> (Format, Result<Loaded, String>) {
    if legacy::is_old_yft_file(contents) {
        let r = legacy::parse_old_yf_file(contents).map(|t| Loaded { tournament: t, warnings: vec![] });
        return (Format::Legacy, r);
    }
    let r = yft::load_yft(contents, cur_version).map(|l| Loaded { tournament: l.tournament, warnings: l.warnings });
    (Format::Yft, r)
}
