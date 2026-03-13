use std::sync::Arc;

use ferrocat::runtime::{
    compile_catalog, compile_icu, DefaultFormatHost, FormatHost, MessageValue, MessageValues,
};
use ferrocat::{
    generate_message_id, Catalog, CatalogEntry, CatalogTranslation, CompileCatalogOptions,
    CompileIcuOptions,
};

mod runtime_compile_icu {
    use super::*;

    #[test]
    fn formats_arguments_lists_and_booleans_with_the_default_host() {
        let compiled = compile_icu(
            "Hello {name}! {items} enabled={enabled}",
            &CompileIcuOptions::new("en"),
        )
        .expect("message should compile");

        let rendered = compiled.format(&MessageValues::from([
            (String::from("name"), MessageValue::from("Sebastian")),
            (
                String::from("items"),
                MessageValue::List(vec![
                    MessageValue::from("alpha"),
                    MessageValue::from("beta"),
                ]),
            ),
            (String::from("enabled"), MessageValue::from(true)),
        ]));

        assert_eq!(rendered, "Hello Sebastian! alpha, beta enabled=true");
    }

    #[test]
    fn uses_the_public_runtime_host_surface_for_locale_and_tag_handling() {
        struct PolishHost;

        impl FormatHost for PolishHost {
            fn locale(&self) -> &str {
                "pl"
            }

            fn format_number(
                &self,
                _name: &str,
                value: &MessageValue,
                _style: Option<&str>,
                _values: &MessageValues,
            ) -> Option<String> {
                match value {
                    MessageValue::Number(number) => Some(format!("n={number:.1}")),
                    _ => None,
                }
            }
        }

        let compiled = compile_icu(
            "{count, plural, one {<b>{count, number}</b> file} few {<b>{count, number}</b> files} other {<b>{count, number}</b> files}}",
            &CompileIcuOptions::new("en"),
        )
        .expect("message should compile");

        let rendered = compiled.format_with_host(
            &MessageValues::from([
                (String::from("count"), MessageValue::from(2usize)),
                (
                    String::from("b"),
                    MessageValue::Tag(Arc::new(|text: &str| format!("[{text}]"))),
                ),
            ]),
            &PolishHost,
        );

        assert_eq!(rendered, "[n=2.0] files");
    }

    #[test]
    fn default_format_host_is_constructible_from_the_runtime_module() {
        let host = DefaultFormatHost::new("de");
        assert_eq!(FormatHost::locale(&host), "de");
    }
}

mod runtime_compile_catalog {
    use super::*;

    #[test]
    fn exposes_lookup_methods_and_formats_messages() {
        let catalog = Catalog::from([(
            String::from("Hello {name}!"),
            CatalogEntry {
                translation: Some(CatalogTranslation::Singular(String::from("Hallo {name}!"))),
                ..CatalogEntry::default()
            },
        )]);

        let compiled = compile_catalog(&catalog, &CompileCatalogOptions::new("de"))
            .expect("catalog should compile");
        let key = generate_message_id("Hello {name}!", None);

        assert!(compiled.has(&key));
        assert!(compiled.get(&key).is_some());
        assert_eq!(compiled.keys(), vec![key.clone()]);
        assert_eq!(compiled.size(), 1);
        assert_eq!(
            compiled.format(
                &key,
                &MessageValues::from([(String::from("name"), MessageValue::from("Sebastian"),)])
            ),
            "Hallo Sebastian!"
        );
    }

    #[test]
    fn returns_the_lookup_key_for_missing_messages() {
        let catalog = Catalog::new();
        let compiled = compile_catalog(&catalog, &CompileCatalogOptions::new("de"))
            .expect("catalog should compile");

        assert_eq!(compiled.format("missing", &MessageValues::new()), "missing");
    }
}
