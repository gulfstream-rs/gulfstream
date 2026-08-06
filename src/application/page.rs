use serde::Serialize;

/// A stable, serializable page of application resources.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total: i64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, page: u32, page_size: u32, total: i64) -> Self {
        Self {
            items,
            page,
            page_size,
            total,
        }
    }
}
