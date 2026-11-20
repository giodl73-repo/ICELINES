use icelines_sources::raw_inventory::inventory;
#[test]
fn array_coverage_distinguishes_absent_null_and_zero() {
    let r=inventory("json",r#"{"plays":[{"typeDescKey":"goal","details":{"target":null,"zero":0}},{"typeDescKey":"giveaway","details":{}}]}"#).unwrap();
    let p = r
        .json_paths
        .iter()
        .find(|p| p.path == "$[\"plays\"][*][\"details\"][\"target\"]")
        .unwrap();
    assert_eq!((p.occurrences, p.null_values), (1, 1));
    assert_eq!(r.observed_plays_tags.unwrap().rows, 2);
    let p = r
        .json_paths
        .iter()
        .find(|p| p.path.ends_with("[\"zero\"]"))
        .unwrap();
    assert_eq!((p.occurrences, p.null_values), (1, 0));
}
#[test]
fn object_keys_and_array_wildcards_do_not_collide() {
    let r = inventory("json", r#"{"x":[1],"x[*]":2,"a\"b":3}"#).unwrap();
    let p = r
        .json_paths
        .iter()
        .map(|p| p.path.as_str())
        .collect::<Vec<_>>();
    assert!(p.contains(&"$[\"x\"][*]"));
    assert!(p.contains(&"$[\"x[*]\"]"));
    assert!(p.contains(&"$[\"a\\\"b\"]"));
}
#[test]
fn csv_keeps_zero_and_null_words_as_nonempty_values() {
    let r = inventory("csv", "id,value\n1,0\n2,\n3,null\n").unwrap();
    assert_eq!(r.csv_rows, Some(3));
    assert_eq!((r.csv_columns[1].nonempty, r.csv_columns[1].empty), (2, 1));
    assert!(inventory("csv", "id,id\n1,2\n").is_err());
}
#[test]
fn raw_byte_fingerprints_change_without_semantic_field_changes() {
    let a = inventory("json", "{}").unwrap();
    let b = inventory("json", " { }\n").unwrap();
    assert_ne!(a.input_sha256, b.input_sha256);
    assert_eq!(a.json_paths, b.json_paths);
    assert!(inventory("json", "{").is_err());
    assert!(inventory("guess", "{}").is_err());
}
#[test]
fn missing_event_tags_and_empty_arrays_are_explicit() {
    let r = inventory(
        "json",
        r#"{"plays":[{"typeDescKey":"goal"},{"typeDescKey":null},{}]}"#,
    )
    .unwrap()
    .observed_plays_tags
    .unwrap();
    assert_eq!(r.missing_or_nonstring_type, 2);
    let r = inventory("json", r#"{"plays":[]}"#)
        .unwrap()
        .observed_plays_tags
        .unwrap();
    assert_eq!(r.rows, 0);
    assert!(r.type_counts.is_empty());
}

#[test]
fn explicit_csv_groups_preserve_empty_category_and_zero_presence() {
    let r = icelines_sources::raw_inventory::csv_categories(
        "kind,target\nA,0\nB,\n,unknown\nA,x\n",
        "kind",
    )
    .unwrap();
    assert_eq!(r.total_rows, 4);
    assert_eq!(r.groups.len(), 3);
    let a = r.groups.iter().find(|g| g.value == "A").unwrap();
    assert_eq!(a.rows, 2);
    assert_eq!(a.columns[1].nonempty, 2);
    let b = r.groups.iter().find(|g| g.value == "B").unwrap();
    assert_eq!(b.columns[1].empty, 1);
    assert!(r.groups.iter().any(|g| g.value.is_empty()));
    assert_eq!(r.groups.iter().map(|g| g.rows).sum::<usize>(), r.total_rows);
}
#[test]
fn csv_grouping_never_guesses_documentation_aliases() {
    assert!(icelines_sources::raw_inventory::csv_categories(
        "game_date,kind\n2021-01-01,A\n",
        "Date"
    )
    .is_err());
}

#[test]
fn csv_category_budget_fails_instead_of_truncating() {
    let mut raw = String::from("kind\n");
    for i in 0..513 {
        raw.push_str(&format!("category-{i}\n"));
    }
    let error = icelines_sources::raw_inventory::csv_categories(&raw, "kind").unwrap_err();
    assert!(error.contains("budget exceeded"));
}
