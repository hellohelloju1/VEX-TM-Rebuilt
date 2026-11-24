use tournament_manager_rs::{display_settings::validate, model::*};
#[test]
fn direct_display_changes_validate_without_touching_matches() {
    let e = Event::default();
    let before = serde_json::to_string(&e.matches).unwrap();
    let mut ds = e.config.displays.clone();
    ds[0].name = "Main Pit".into();
    assert!(validate(&e, &ds, &[]).is_ok());
    assert_eq!(before, serde_json::to_string(&e.matches).unwrap());
    ds[1].id = ds[0].id;
    assert!(validate(&e, &ds, &[]).is_err());
}
#[test]
fn invalid_field_and_logo_data_are_rejected() {
    let e = Event::default();
    let mut ds = e.config.displays.clone();
    ds[1].field = "Missing".into();
    assert!(validate(&e, &ds, &[]).is_err());
    let logos = [Sponsor {
        name: "Bad".into(),
        image: vec![1, 2, 3],
    }];
    assert!(validate(&e, &e.config.displays, &logos).is_err());
}
#[test]
fn bundled_sponsor_image_is_accepted() {
    let e = Event::default();
    let logos = [Sponsor {
        name: "Test".into(),
        image: include_bytes!("../assets/v5.png").to_vec(),
    }];
    assert!(validate(&e, &e.config.displays, &logos).is_ok());
}

#[test]
fn live_pit_screen_switch_validates_and_persists_without_changing_matches() {
    use tournament_manager_rs::{display_settings::pit_screen,store};
    let mut e=Event::default();let before=serde_json::to_value(&e.matches).unwrap();let id=e.config.displays.iter().find(|d|d.kind=="Pit").unwrap().id;
    pit_screen(&mut e,id,"Skills").unwrap();
    assert_eq!(e.config.displays.iter().find(|d|d.id==id).unwrap().screen,"Skills");
    assert!(pit_screen(&mut e,id,"Other").is_err());
    let field=e.config.displays.iter().find(|d|d.kind=="Field").unwrap().id;
    assert!(pit_screen(&mut e,field,"Skills").is_err());
    assert_eq!(serde_json::to_value(&e.matches).unwrap(),before);
    let path=std::env::temp_dir().join(format!("tmrs-pit-toggle-{}.tmrs",std::process::id()));store::save(&path,&e).unwrap();assert_eq!(store::load(&path).unwrap().config.displays.iter().find(|d|d.id==id).unwrap().screen,"Skills");std::fs::remove_file(path).unwrap();
    pit_screen(&mut e,id,"Rankings").unwrap();
}

#[test]
fn recent_qual_results_include_only_last_two_published_qualifications() {
    use tournament_manager_rs::{display_settings::recent_qual_results,scoring::Game};
    let mut e=Event::default();let g=Game::all().into_iter().find(|g|g.id()==e.game).unwrap();
    for (id,round,state,pins) in [(1,2,MatchState::Submitted,1),(2,2,MatchState::Scored,7),(3,2,MatchState::Submitted,3),(4,2,MatchState::Submitted,2),(8,3,MatchState::Submitted,9),(9,1,MatchState::Submitted,8)] {let mut m=g.new_match(id,round,&[1,2,3,4]);m.number=id;m.state=state;m.values.insert("red_quad_1_red_pins".into(),pins);e.matches.push(m);}
    let results=recent_qual_results(&e);assert_eq!(results.len(),2);assert_eq!(results[0]["match"]["number"],4);assert_eq!(results[1]["match"]["number"],3);assert_eq!(results[0]["score"]["alliances"][0]["score"].as_f64(),Some(10.));
    e.matches[3].state=MatchState::Scored;let results=recent_qual_results(&e);assert_eq!(results[0]["match"]["number"],3);assert_eq!(results[1]["match"]["number"],1);
}
