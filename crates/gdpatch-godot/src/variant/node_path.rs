use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NodePath {
    path: Vec<String>,
    subpath: Vec<String>,
    absolute: bool,
}

impl NodePath {
    pub fn new(path: Vec<String>, subpath: Vec<String>, absolute: bool) -> NodePath {
        Self {
            path,
            subpath,
            absolute,
        }
    }

    pub fn names(&self) -> &[String] {
        &self.path
    }

    pub fn sub_names(&self) -> &[String] {
        &self.subpath
    }

    pub fn is_absolute(&self) -> bool {
        self.absolute
    }
}

impl Display for NodePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();

        if self.absolute {
            result.push('/');
        }

        result += &self.path.join("/");

        if !self.subpath.is_empty() {
            result.push(':');
            result += &self.subpath.join("/");
        }

        write!(f, "{}", result)
    }
}

/// Error type returned by [`NodePath::from_str`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[error("invalid node path")]
pub struct FromStrError;

impl FromStr for NodePath {
    type Err = FromStrError;

    fn from_str(mut path: &str) -> Result<Self, Self::Err> {
        if path.is_empty() {
            return Ok(Default::default());
        }

        let path_chars = path.chars().collect::<Vec<_>>();
        let absolute = path_chars[0] == '/';

        let mut subpath = Vec::new();
        let mut slices = 0;
        let mut last_is_slash = true;

        if let Some(subpath_pos) = path.find(':') {
            let mut from = subpath_pos + 1;

            let start = from;
            for i in start..=path_chars.len() {
                if matches!(path_chars.get(i), Some(':') | None) {
                    let str = &path[from..i - from];

                    if str.is_empty() {
                        if path_chars.get(i).is_none() {
                            continue; // Allow end-of-path :
                        }

                        return Err(FromStrError);
                    }

                    subpath.push(str.to_string());
                    from = i + 1;
                }
            }

            path = &path[0..subpath_pos];
        }

        for ch in path_chars.iter().skip(absolute as _) {
            if *ch == '/' {
                last_is_slash = true;
            } else {
                if last_is_slash {
                    slices += 1;
                }

                last_is_slash = false;
            }
        }

        if slices == 0 && !absolute && subpath.is_empty() {
            return Err(FromStrError);
        }

        if slices == 0 {
            return Ok(Self {
                path: Vec::new(),
                subpath,
                absolute,
            });
        }

        let mut out_path = Vec::with_capacity(slices);
        last_is_slash = true;

        let slice = 0;
        let mut from = absolute as usize;

        for i in (absolute as _)..=path_chars.len() {
            if matches!(path_chars.get(i), Some('/') | None) {
                if !last_is_slash {
                    let name = &path[from..i - from];
                    assert!(slice <= slices);
                    out_path.push(name.to_string());
                }

                from = i + 1;
                last_is_slash = true;
            } else {
                last_is_slash = false;
            }
        }

        Ok(Self {
            path: out_path,
            subpath,
            absolute,
        })
    }
}
