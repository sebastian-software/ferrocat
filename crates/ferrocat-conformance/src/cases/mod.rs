mod icu_official;
mod po_babel;
mod po_pofile;
mod po_polib;

use crate::ConformanceManifest;

pub fn all_manifests() -> Vec<ConformanceManifest> {
    vec![
        po_polib::manifest(),
        po_pofile::manifest(),
        po_babel::manifest(),
        icu_official::manifest(),
    ]
}
