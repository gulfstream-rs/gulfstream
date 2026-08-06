use crate::config::ApiConfig;

#[derive(Clone, Copy, Debug)]
pub struct Pagination {
    pub page: u32,
    pub page_size: u32,
}

impl Pagination {
    pub fn from_query(page: Option<u32>, page_size: Option<u32>, config: &ApiConfig) -> Self {
        Self {
            page: page.unwrap_or(1).max(1),
            page_size: page_size
                .unwrap_or(config.default_page_size)
                .clamp(1, config.maximum_page_size),
        }
    }

    pub fn offset(self) -> i64 {
        i64::from(self.page.saturating_sub(1)) * i64::from(self.page_size)
    }
}

#[cfg(test)]
mod tests {
    use super::Pagination;
    use crate::config::ApiConfig;

    fn config() -> ApiConfig {
        ApiConfig {
            default_page_size: 25,
            maximum_page_size: 100,
        }
    }

    #[test]
    fn normalizes_page_and_page_size() {
        let pagination = Pagination::from_query(Some(0), Some(500), &config());
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.page_size, 100);
        assert_eq!(pagination.offset(), 0);
    }

    #[test]
    fn calculates_offset_without_underflow() {
        let pagination = Pagination::from_query(Some(3), Some(10), &config());
        assert_eq!(pagination.offset(), 20);
    }
}
