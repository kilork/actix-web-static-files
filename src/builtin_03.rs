use std::collections::HashMap;

use static_files_03::Resource;

use crate::{ResourceFile, ResourceFilesCollection};

impl ResourceFile for Resource {
    fn data(&self) -> &'static [u8] {
        self.data
    }

    fn modified(&self) -> u64 {
        self.modified
    }

    fn mime_type(&self) -> &str {
        self.mime_type
    }
}

impl ResourceFilesCollection for HashMap<&'static str, Resource> {
    type Resource = Resource;
    fn get_resource(&self, path: &str) -> Option<&Self::Resource> {
        self.get(path)
    }

    fn contains_key(&self, path: &str) -> bool {
        self.contains_key(path)
    }
}
