use std::path::{Path, PathBuf};

use red4ext_sys::error::ResourcePathError;
use red4ext_sys::interop::ResourcePath;

#[derive(Default)]
pub struct ResourcePathBuilder {
    components: PathBuf,
}

impl ResourcePathBuilder {
    pub fn join(mut self, component: impl AsRef<Path>) -> Self {
        self.components.push(component);
        self
    }

    pub fn build(self) -> Result<ResourcePath, ResourcePathError> {
        ResourcePath::new(&self.components.to_string_lossy())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use red4ext_sys::interop::ResourcePath;

    use crate::prelude::ResourcePathBuilder;

    #[test]
    fn resource_path_builder() {
        assert_eq!(
            ResourcePathBuilder::default()
                .join("base")
                .join("somewhere")
                .join("in")
                .join("archive")
                .build()
                .unwrap(),
            ResourcePath::new("base\\somewhere\\in\\archive").unwrap()
        );

        let path = PathBuf::from("multi\\").join("somewhere");
        assert_eq!(
            ResourcePathBuilder::default()
                .join(path)
                .join("in")
                .join("archive")
                .build()
                .unwrap(),
            ResourcePath::new("multi\\somewhere\\in\\archive").unwrap()
        );
    }
}
