use tournament_manager_rs::store;

#[test]
fn imports_utf8_and_legacy_mac_roman_without_losing_names() {
    let path = std::env::temp_dir().join(format!("tmrs-csv-{}.csv", std::process::id()));
    for (bytes, expected) in [
        (b"Number,Name,Organization\r\n82T,K\xaeL\xafN,School\r\n".to_vec(), "KÆLØN"),
        ("\u{feff}Number,Name,Organization\n82T,北京机器人,School\n".as_bytes().to_vec(), "北京机器人"),
        ("Number,Name,Organization\n82T,KÆLØN,School\n".as_bytes().to_vec(), "KÆLØN"),
    ] {
        std::fs::write(&path, bytes).unwrap();
        let teams = store::import_csv(&path, 7).unwrap();
        assert_eq!(teams.len(), 1);
        assert_eq!(teams[0].id, 7);
        assert_eq!(teams[0].number, "82T");
        assert_eq!(teams[0].name, expected);
        assert_eq!(teams[0].organization, "School");
    }
    std::fs::remove_file(path).unwrap();
}
