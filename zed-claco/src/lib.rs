use zed_extension_api as zed;

struct ClacoExtension;

impl zed::Extension for ClacoExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(ClacoExtension);
