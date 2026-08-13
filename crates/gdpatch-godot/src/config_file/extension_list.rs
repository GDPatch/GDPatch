//! The extension list, which tracks enabled GDExtensions.
//! Despite using the ".cfg" file extension, it is a simple text file, with one path per line.
#[derive(Debug, Clone, Default)]
pub struct ExtensionList(pub Vec<String>);

impl ExtensionList {
    pub const EXTENSION_LIST_PATH: &str = ".godot/extension_list.cfg";

    pub fn parse(data: &str) -> Self {
        Self(
            data.trim()
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
        )
    }

    pub fn write(&mut self) -> String {
        self.0.join("\n")
    }

    pub fn merge(&mut self, other: &Self) {
        self.0.extend_from_slice(&other.0);
    }

    pub fn merge_decode(&mut self, str: &str) {
        let other = Self::parse(str);
        self.merge(&other);
    }
}
