use tournament_manager_rs::{model::*,scoring::Game};
fn game(id: &str)->Game { Game::all().into_iter().find(|g|g.id()==id).unwrap() }
#[test]
fn level_up_goal_values_constraints_and_skills() {
    let g=game("VIQC_2026_LevelUp");
    let mut m=g.new_match(1,2,&[1,2]);
    for (key,value) in [("red_goal_floor",1),("blue_goal_level_1",2),("red_goal_level_2",3),("blue_goal_level_3",1),("goal_level_4",2)] { m.values.insert(key.into(),value); }
    assert!(g.constraints(&m).unwrap().is_empty());
    let score=g.score(&m).unwrap();
    for a in 0..2 { assert_eq!(score["alliances"][a]["score"].as_f64(),Some(69.)); }
    m.values.insert("red_goal_floor".into(),23);
    assert!(!g.constraints(&m).unwrap().is_empty());
    let (shared,values)=g.defaults(true);
    m.values=shared;m.alliances=vec![AllianceInput{teams:vec![Slot{team:1,..Default::default()}],values:values[0].clone()}];
    m.alliances[0].values.insert("goal_level_4".into(),2);
    assert_eq!(g.run("calc_skills_score",&m,Some(0)).unwrap()["alliances"][0]["score"].as_f64(),Some(32.));
}
#[test]
fn override_ownership_midfield_autonomous_and_dq() {
    let g=game("VRC_2026_Override");let mut m=g.new_match(1,2,&[1,2,3,4]);
    for (key,value) in [("red_quad_1_red_pins",3),("center_blue_pins",2),("red_quad_1_yellow_pins",4),("red_quad_1_toggle",1),("center_yellow_pins",3)] {m.values.insert(key.into(),value);}
    m.alliances[0].values.insert("midfield_robots".into(),1);m.alliances[0].values.insert("auto".into(),1);
    assert!(g.constraints(&m).unwrap().is_empty());
    let score=g.score(&m).unwrap();assert_eq!(score["alliances"][0]["score"].as_f64(),Some(105.));assert_eq!(score["alliances"][1]["score"].as_f64(),Some(10.));
    m.alliances[1].values.insert("auto".into(),1);assert!(!g.constraints(&m).unwrap().is_empty());m.alliances[1].values.insert("auto".into(),0);
    m.round=3;m.alliances[0].teams[0].dq=true;assert_eq!(g.score(&m).unwrap()["alliances"][0]["score"].as_f64(),Some(0.));
}
#[test]
fn current_season_defaults_and_legacy_tournaments_remain_supported() {
    assert_eq!(Event::default().game,"VRC_2026_Override");
    for id in ["VIQC_2026_LevelUp","VRC_2026_Override"] {let g=game(id);assert_eq!(g.data["season"],"2026/2027");assert!(!g.controls(true).is_empty());assert!(!g.controls(false).is_empty());}
    for id in ["VIQC_2025_MixAndMatch","VRC_2025_PushBack"] {assert_eq!(game(id).data["season"],"2025/2026");}
}

#[test]
fn override_skills_uses_ownership_and_midfield_points() {
    let g=game("VRC_2026_Override");let mut m=g.new_match(1,30,&[1]);
    let (shared,values)=g.defaults(true);m.values=shared;m.alliances=vec![AllianceInput{teams:vec![Slot{team:1,..Default::default()}],values:values[0].clone()}];
    for (key,value) in [("red_quad_1_red_pins",1),("red_quad_1_yellow_pins",2),("red_quad_1_toggle",1),("midfield_robots",1),("center_yellow_pins",1)] {m.alliances[0].values.insert(key.into(),value);}
    for skill in 0..2 {assert_eq!(g.run("calc_skills_score",&m,Some(skill)).unwrap()["alliances"][0]["score"].as_f64(),Some(43.));assert!(g.run("check_skills_constraints",&m,Some(skill)).unwrap().is_null());}
}
