use serde::Serialize;

#[derive(Serialize)]
pub struct PostContext {
    pub title: String,
    pub relative_url: String,

    pub date: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,

    pub markdown: String,
}
