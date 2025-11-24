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
        EverythingSortKind::NameAscending
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
            sort_method: Some(self.sort_method),
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
        self.inner.read().sort_method
    }

    pub fn get_result_limit(&self) -> usize {
        self.inner.read().result_limit
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EverythingSortKind {
    NameAscending,
    NameDescending,
    PathAscending,
    PathDescending,
    SizeAscending,
    SizeDescending,
    ExtensionAscending,
    ExtensionDescending,
    TypeNameAscending,
    TypeNameDescending,
    DateCreatedAscending,
    DateCreatedDescending,
    DateModifiedAscending,
    DateModifiedDescending,
    AttributesAscending,
    AttributesDescending,
    FileListFilenameAscending,
    FileListFilenameDescending,
    RunCountAscending,
    RunCountDescending,
    DateRecentlyChangedAscending,
    DateRecentlyChangedDescending,
    DateAccessedAscending,
    DateAccessedDescending,
    DateRunAscending,
    DateRunDescending,
}

impl From<EverythingSortKind> for EverythingSort {
    fn from(kind: EverythingSortKind) -> Self {
        match kind {
            EverythingSortKind::NameAscending => EverythingSort::NameAscending,
            EverythingSortKind::NameDescending => EverythingSort::NameDescending,
            EverythingSortKind::PathAscending => EverythingSort::PathAscending,
            EverythingSortKind::PathDescending => EverythingSort::PathDescending,
            EverythingSortKind::SizeAscending => EverythingSort::SizeAscending,
            EverythingSortKind::SizeDescending => EverythingSort::SizeDescending,
            EverythingSortKind::ExtensionAscending => EverythingSort::ExtensionAscending,
            EverythingSortKind::ExtensionDescending => EverythingSort::ExtensionDescending,
            EverythingSortKind::TypeNameAscending => EverythingSort::TypeNameAscending,
            EverythingSortKind::TypeNameDescending => EverythingSort::TypeNameDescending,
            EverythingSortKind::DateCreatedAscending => EverythingSort::DateCreatedAscending,
            EverythingSortKind::DateCreatedDescending => EverythingSort::DateCreatedDescending,
            EverythingSortKind::DateModifiedAscending => EverythingSort::DateModifiedAscending,
            EverythingSortKind::DateModifiedDescending => EverythingSort::DateModifiedDescending,
            EverythingSortKind::AttributesAscending => EverythingSort::AttributesAscending,
            EverythingSortKind::AttributesDescending => EverythingSort::AttributesDescending,
            EverythingSortKind::FileListFilenameAscending => {
                EverythingSort::FileListFilenameAscending
            }
            EverythingSortKind::FileListFilenameDescending => {
                EverythingSort::FileListFilenameDescending
            }
            EverythingSortKind::RunCountAscending => EverythingSort::RunCountAscending,
            EverythingSortKind::RunCountDescending => EverythingSort::RunCountDescending,
            EverythingSortKind::DateRecentlyChangedAscending => {
                EverythingSort::DateRecentlyChangedAscending
            }
            EverythingSortKind::DateRecentlyChangedDescending => {
                EverythingSort::DateRecentlyChangedDescending
            }
            EverythingSortKind::DateAccessedAscending => EverythingSort::DateAccessedAscending,
            EverythingSortKind::DateAccessedDescending => EverythingSort::DateAccessedDescending,
            EverythingSortKind::DateRunAscending => EverythingSort::DateRunAscending,
            EverythingSortKind::DateRunDescending => EverythingSort::DateRunDescending,
        }
    }
}
