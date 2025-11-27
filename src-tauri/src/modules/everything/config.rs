#[cfg(target_arch = "x86_64")]
use everything_rs::EverythingSort;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialEverythingConfig {
    pub sort_threshold: Option<usize>,
    pub sort_method: Option<EverythingSortKind>,
    pub result_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EverythingConfigInner {
    #[serde(default = "EverythingConfigInner::default_sort_threshold")]
    pub sort_threshold: usize,
    #[serde(default = "EverythingConfigInner::default_sort_method")]
    pub sort_method: EverythingSortKind,
    #[serde(default = "EverythingConfigInner::default_result_limit")]
    pub result_limit: usize,
}

impl Default for EverythingConfigInner {
    fn default() -> Self {
        Self {
            sort_threshold: Self::default_sort_threshold(),
            sort_method: Self::default_sort_method(),
            result_limit: Self::default_result_limit(),
        }
    }
}

impl EverythingConfigInner {
    pub(crate) fn default_sort_threshold() -> usize {
        3
    }

    pub(crate) fn default_sort_method() -> EverythingSortKind {
        EverythingSortKind("NameAscending".to_string())
    }

    pub(crate) fn default_result_limit() -> usize {
        10
    }

    pub fn update(&mut self, partial: PartialEverythingConfig) {
        if let Some(sort_threshold) = partial.sort_threshold {
            self.sort_threshold = sort_threshold;
        }
        if let Some(sort_method) = partial.sort_method {
            self.sort_method = sort_method;
        }
        if let Some(result_limit) = partial.result_limit {
            self.result_limit = result_limit;
        }
    }

    pub fn to_partial(&self) -> PartialEverythingConfig {
        PartialEverythingConfig {
            sort_threshold: Some(self.sort_threshold),
            sort_method: Some(self.sort_method.clone()),
            result_limit: Some(self.result_limit),
        }
    }
}

#[derive(Debug)]
pub struct EverythingConfig {
    inner: RwLock<EverythingConfigInner>,
}

impl Default for EverythingConfig {
    fn default() -> Self {
        Self {
            inner: RwLock::new(EverythingConfigInner::default()),
        }
    }
}

impl EverythingConfig {
    pub fn update(&self, partial: PartialEverythingConfig) {
        let mut inner = self.inner.write();
        inner.update(partial);
    }

    pub fn to_partial(&self) -> PartialEverythingConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_sort_threshold(&self) -> usize {
        self.inner.read().sort_threshold
    }

    pub fn get_sort_method(&self) -> EverythingSortKind {
        self.inner.read().sort_method.clone()
    }

    pub fn get_result_limit(&self) -> usize {
        self.inner.read().result_limit
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EverythingSortKind(String);

#[cfg(target_arch = "x86_64")]
impl From<EverythingSortKind> for EverythingSort {
    fn from(kind: EverythingSortKind) -> Self {
        match kind.0.as_str() {
            "NameAscending" => EverythingSort::NameAscending,
            "NameDescending" => EverythingSort::NameDescending,
            "PathAscending" => EverythingSort::PathAscending,
            "PathDescending" => EverythingSort::PathDescending,
            "SizeAscending" => EverythingSort::SizeAscending,
            "SizeDescending" => EverythingSort::SizeDescending,
            "ExtensionAscending" => EverythingSort::ExtensionAscending,
            "ExtensionDescending" => EverythingSort::ExtensionDescending,
            "TypeNameAscending" => EverythingSort::TypeNameAscending,
            "TypeNameDescending" => EverythingSort::TypeNameDescending,
            "DateCreatedAscending" => EverythingSort::DateCreatedAscending,
            "DateCreatedDescending" => EverythingSort::DateCreatedDescending,
            "DateModifiedAscending" => EverythingSort::DateModifiedAscending,
            "DateModifiedDescending" => EverythingSort::DateModifiedDescending,
            "AttributesAscending" => EverythingSort::AttributesAscending,
            "AttributesDescending" => EverythingSort::AttributesDescending,
            "FileListFilenameAscending" => EverythingSort::FileListFilenameAscending,
            "FileListFilenameDescending" => EverythingSort::FileListFilenameDescending,
            "RunCountAscending" => EverythingSort::RunCountAscending,
            "RunCountDescending" => EverythingSort::RunCountDescending,
            "DateRecentlyChangedAscending" => EverythingSort::DateRecentlyChangedAscending,
            "DateRecentlyChangedDescending" => EverythingSort::DateRecentlyChangedDescending,
            "DateAccessedAscending" => EverythingSort::DateAccessedAscending,
            "DateAccessedDescending" => EverythingSort::DateAccessedDescending,
            "DateRunAscending" => EverythingSort::DateRunAscending,
            "DateRunDescending" => EverythingSort::DateRunDescending,
            _ => EverythingSort::NameAscending, // 默认值
        }
    }
}
