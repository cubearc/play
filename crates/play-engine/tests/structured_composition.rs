use play_engine::worker;
#[test]
fn native_catalog_exposes_core_owned_field_and_numeric_operations() {
    let catalog = worker::catalog().unwrap();
    let entries = catalog.as_array().unwrap();
    for id in [
        "json.parse",
        "json.select",
        "json.text",
        "text.integer",
        "number.maximum",
        "number.add",
        "image.dimensions",
    ] {
        let op = entries
            .iter()
            .find(|v| v["operation"] == id)
            .unwrap_or_else(|| panic!("missing native adapter {id}"));
        assert_eq!(op["operation_version"], 1);
        assert_eq!(op["contract"]["operation"], id);
    }
}
