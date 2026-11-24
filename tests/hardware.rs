use tournament_manager_rs::hardware::V5;
#[test]
fn v5_commands_and_watchdog_use_fake_driver_only() {
    let (Ok(helper), Ok(driver)) = (
        std::env::var("TMRS_TEST_V5_HELPER"),
        std::env::var("TMRS_TEST_V5_DRIVER"),
    ) else {
        return;
    };
    let mut hw = V5::spawn(std::path::Path::new(&helper), std::path::Path::new(&driver)).unwrap();
    assert_eq!(hw.count, 1);
    hw.connect(0).unwrap();
    assert_eq!(hw.status().unwrap()["Mode"], "idle");
    hw.prepare(2, 1, 1).unwrap();
    hw.control("auto", 15).unwrap();
    assert_eq!(hw.status().unwrap()["Mode"], "auto");
    hw.control("disable", 0).unwrap();
    assert_eq!(hw.status().unwrap()["Mode"], "disable");
    hw.control("driver", 105).unwrap();
    assert_eq!(hw.status().unwrap()["Mode"], "driver");
    std::thread::sleep(std::time::Duration::from_millis(2300));
    assert!(hw.status().unwrap_err().to_string().contains("watchdog"));
    hw.disconnect();
}
