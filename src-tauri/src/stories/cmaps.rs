pub mod default;
pub mod easy;

#[macro_export]
macro_rules! rust_json {
    ($($json:tt)+) => {
        use $crate::config_handles::JsonMap;
        use once_cell::sync::Lazy;
        use serde_json::json;

        static JSON: Lazy<JsonMap> = Lazy::new(|| json!({ $($json)+ }).as_object().unwrap().clone());
        pub fn json() -> JsonMap {
            JSON.clone()
        }
    };
}
