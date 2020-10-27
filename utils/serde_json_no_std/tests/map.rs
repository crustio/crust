use serde_json::{from_str, Map, Value};

#[test]
fn test_preserve_order() {
    // Sorted order
    #[cfg(not(feature = "preserve_order"))]
    const EXPECTED: &[&str] = &["a", "b", "c"];

    // Insertion order
    #[cfg(feature = "preserve_order")]
    const EXPECTED: &[&str] = &["b", "a", "c"];

    let v: Value = from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
    let keys: Vec<_> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, EXPECTED);
}

#[test]
fn test_append() {
    // Sorted order
    #[cfg(not(feature = "preserve_order"))]
    const EXPECTED: &[&str] = &["a", "b", "c"];

    // Insertion order
    #[cfg(feature = "preserve_order")]
    const EXPECTED: &[&str] = &["b", "a", "c"];

    let mut v: Value = from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
    let val = v.as_object_mut().unwrap();
    let mut m = Map::new();
    m.append(val);
    let keys: Vec<_> = m.keys().collect();

    assert_eq!(keys, EXPECTED);
    assert!(val.is_empty());
}
