use crate::normalization::NormalizedBaseBar;

#[derive(Debug)]
pub struct HistoricalWindow {
    pub bars: Vec<NormalizedBaseBar>,
    pub metadata_time: Vec<(i64, String)>,
}

#[derive(Debug)]
pub struct SymbolSearchResult {
    pub dataset: String,
    pub symbol: String,
    pub code: i64,
}
