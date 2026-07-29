use super::*;

#[test]
fn cached_icu_plural_categories_reads_poisoned_cache_entries() {
    let cache = Mutex::new(HashMap::new());
    let _ = std::panic::catch_unwind(|| {
        let mut guard = cache.lock().expect("lock");
        guard.insert("fr".to_owned(), Some(vec!["one", "many", "other"]));
        panic!("poison cache");
    });

    let categories = cached_icu_plural_categories_for("fr", &cache);
    assert_eq!(categories, Some(vec!["one", "many", "other"]));
}

#[test]
fn cached_icu_plural_categories_computes_with_poisoned_cache() {
    let cache = Mutex::new(HashMap::new());
    let _ = std::panic::catch_unwind(|| {
        let _guard = cache.lock().expect("lock");
        panic!("poison cache");
    });

    let categories = cached_icu_plural_categories_for("de", &cache);
    assert!(categories.is_some());
    assert!(categories.expect("categories").contains(&"other"));
}
