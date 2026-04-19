use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename = "radiko")]
pub struct RadikoProgramXml {
    pub ttl: Option<u32>,
    pub srvtime: Option<u64>,
    pub stations: StationsXml,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StationsXml {
    #[serde(rename = "station")]
    pub station: Vec<StationXml>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProgramsXml {
    pub date: Option<String>,
    #[serde(rename = "prog")]
    /// 週の変わり目（日曜日？）でAPIからprogタグが空で返ってくるので注意
    /// <progs>
    ///     <date>20000101</date>
    /// </progs>
    /// のようにレスポンスXMLの末尾で<prog></prog>が欠落した状態になっている
    pub program: Option<Vec<ProgramXml>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProgramXml {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@master_id")]
    pub master_id: Option<String>,
    #[serde(rename = "@ft")]
    pub ft: String,
    #[serde(rename = "@to")]
    pub to: String,
    #[serde(rename = "@ftl")]
    pub ftl: String,
    #[serde(rename = "@tol")]
    pub tol: String,
    #[serde(rename = "@dur")]
    pub dur: Option<u32>,

    pub title: String,
    pub url: Option<String>,
    pub desc: Option<String>,
    pub url_link: Option<String>,
    pub info: Option<String>,
    pub pfm: Option<String>,
    pub img: Option<String>,
    pub failed_record: Option<u8>,
    pub ts_in_ng: Option<u8>,
    pub tsplus_in_ng: Option<u8>,
    pub ts_out_ng: Option<u8>,
    pub tsplus_out_ng: Option<u8>,
    pub tag: Option<TagXml>,
    pub genre: Option<GenreXml>,
    pub metas: Option<MetasXml>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StationXml {
    #[serde(rename = "@id")]
    pub id: String,
    pub name: String,
    #[serde(rename = "progs")]
    pub programs: Vec<ProgramsXml>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TagXml {
    #[serde(rename = "item", default)]
    pub items: Vec<TagItemXml>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TagItemXml {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GenreXml {
    #[serde(rename = "program", default)]
    pub programs: Vec<GenreProgramXml>,
    #[serde(rename = "personality", default)]
    pub personalities: Vec<GenrePersonalityXml>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GenreProgramXml {
    #[serde(rename = "@id")]
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GenrePersonalityXml {
    #[serde(rename = "@id")]
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MetasXml {
    #[serde(rename = "meta", default)]
    pub metas: Vec<MetaXml>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MetaXml {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: String,
}
