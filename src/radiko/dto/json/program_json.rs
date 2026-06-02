use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RootJson {
    pub meta: Option<MetaJson>,
    pub data: Option<Vec<ProgramJson>>,
}

#[derive(Debug, Deserialize)]
pub struct MetaJson {
    pub key: Vec<String>,
    pub station_id: Vec<String>,
    pub area_id: Vec<String>,
    pub cur_area_id: String,
    pub region_id: String,
    pub start_day: String,
    pub end_day: String,
    pub filter: String,
    pub result_count: u32,
    pub page_idx: u32,
    pub row_limit: u32,
    pub kakuchou: Vec<String>,
    pub suisengo: String,
    pub genre_id: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProgramJson {
    pub start_time: String,
    pub end_time: String,
    pub start_time_s: String,
    pub end_time_s: String,
    pub program_date: String,
    pub program_url: String,
    pub station_id: String,
    pub performer: String,
    pub title: String,
    pub info: String,
    pub description: String,
    pub status: String,
    pub img: String,
    pub genre: GenreJson,
    pub ts_in_ng: u8,
    pub ts_out_ng: u8,
    pub tsplus_in_ng: u8,
    pub tsplus_out_ng: u8,
    pub metas: Vec<MetaItemJson>,
}

#[derive(Debug, Deserialize, Default)]
pub struct GenreJson {
    pub personality: Option<GenreItemJson>,
    pub program: Option<GenreItemJson>,
}

#[derive(Debug, Deserialize)]
pub struct GenreItemJson {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct MetaItemJson {
    pub name: String,
    pub value: String,
}
