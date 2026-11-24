use tournament_manager_rs::{model::*, reports, scoring::Game};
#[test]
fn schedules_are_printable_by_time_and_team() {
    let mut e = Event::default();
    for id in 1..=4 {
        e.teams.push(Team {
            id,
            number: format!("{id}A"),
            name: format!("Team {id}"),
            ..Default::default()
        });
    }
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    let mut late = g.new_match(1, 2, &[1, 2, 3, 4]);
    late.planned = "10:00".into();
    let mut early = g.new_match(2, 1, &[1, 3, 2, 4]);
    early.planned = "09:00".into();
    e.matches = vec![late, early];
    let time = reports::schedule(&e, false);
    assert!(time.contains("window.print()"));
    assert!(time.find("09:00").unwrap() < time.find("10:00").unwrap());
    let team = reports::schedule(&e, true);
    for n in ["1A", "2A", "3A", "4A"] {
        assert!(team.contains(&format!("<h2>Team {n}")));
    }
    e.name = "<script>alert(1)</script>".into();
    assert!(!reports::schedule(&e, true).contains("<script>"));
}

#[test]
fn pdf_report_data_covers_requested_tables_and_distinguishes_provisional_rankings() {
    use tournament_manager_rs::{reports::pdf_data,model::*,scoring::Game};
    let mut e=Event::default();for id in 1..=8{e.teams.push(Team{id,number:format!("{id}A"),name:format!("Named team {id}"),..Default::default()});}
    let g=Game::all().into_iter().find(|g|g.id()==e.game).unwrap();e.matches.push(g.new_match(1,2,&[1,2,3,4]));
    for kind in ["time","team","teams","rankings","skills","results","awards"]{let d=pdf_data(&e,kind).unwrap();for section in d["sections"].as_array().unwrap(){let n=section["columns"].as_array().unwrap().len();assert_eq!(section["widths"].as_array().unwrap().len(),n);let sum:f64=section["widths"].as_array().unwrap().iter().map(|n|n.as_f64().unwrap()).sum();assert!((sum-1.).abs()<0.001);for row in section["rows"].as_array().unwrap(){assert_eq!(row.as_array().unwrap().len(),n);}}}
    assert!(pdf_data(&e,"rankings").unwrap()["title"].as_str().unwrap().contains("provisional"));e.matches[0].state=MatchState::Submitted;assert_eq!(pdf_data(&e,"rankings").unwrap()["title"],"Final qualification rankings");
    e.alliances=vec![vec![1,2],vec![3,4],vec![5,6],vec![7,8]];tournament_manager_rs::bracket::start(&mut e,&g,1).unwrap();let d=pdf_data(&e,"bracket").unwrap();assert!(!d["bracket"].to_string().contains("Named team"));assert!(d["bracket"].to_string().contains("1A"));assert!(d["bracket"].to_string().contains("TBD"));
}

#[test]
fn organization_highlights_match_across_team_list_and_both_schedules() {
    use tournament_manager_rs::reports::pdf_data;
    let mut e=Event::default();
    for (id,org) in [(1," ISB  Robotics "),(2,"isb robotics"),(3,"Other School"),(4,"")] {e.teams.push(Team{id,number:format!("{id}A"),name:format!("Team {id}"),organization:org.into(),..Default::default()});}
    let g=Game::all().into_iter().find(|g|g.id()==e.game).unwrap();let mut m=g.new_match(1,2,&[1,3,2,4]);m.planned="09:00".into();e.matches.push(m);
    let list=pdf_data(&e,"teams").unwrap();let colors=list["sections"][0]["row_colors"].as_array().unwrap();assert_eq!(colors[0],colors[1]);assert_ne!(colors[0],colors[2]);assert!(colors[3].is_null());
    for kind in ["time","team"] {let d=pdf_data(&e,kind).unwrap();let cell=&d["sections"][0]["rows"][0][3];assert_eq!(cell["text"],"1A / 3A vs 2A / 4A");let runs=cell["runs"].as_array().unwrap();for (number,index) in [("1A",0),("2A",1),("3A",2)]{assert_eq!(runs.iter().find(|r|r["text"]==number).unwrap()["background"],colors[index]);}assert!(runs.iter().find(|r|r["text"]=="4A").unwrap().get("background").is_none());assert_eq!(d["sections"].as_array().unwrap().last().unwrap()["name"],"Organizations");}
    let html=reports::schedule(&e,false);assert!(html.contains("class=\"org-team\""));assert!(html.contains("Organizations"));
    if let Ok(out)=std::env::var("TMRS_PDF_QA_DIR"){std::fs::create_dir_all(&out).unwrap();for kind in ["teams","time","team"]{std::fs::write(std::path::Path::new(&out).join(format!("{kind}.json")),serde_json::to_vec(&pdf_data(&e,kind).unwrap()).unwrap()).unwrap();}}
}

#[test]
fn windows_pdf_html_preserves_font_colors_unicode_and_number_only_brackets(){
    use tournament_manager_rs::{model::*,scoring::Game};
    let mut e=Event::default();e.name="<Event>".into();
    for id in 1..=8{e.teams.push(Team{id,number:format!("{id}A"),name:"北京 <script>".into(),organization:if id%2==0{"School B"}else{"School A"}.into(),..Default::default()});}
    let g=Game::all().into_iter().find(|g|g.id()==e.game).unwrap();e.matches.push(g.new_match(1,2,&[1,2,3,4]));
    for kind in ["time","team","teams","rankings","skills","results","awards"]{let html=reports::pdf_html(&e,kind).unwrap();assert!(html.contains("data:font/ttf;base64,"));assert!(html.contains("#812bbb"));assert!(html.contains("border:1px solid"));assert!(html.contains("&lt;Event&gt;"));assert!(!html.contains("<script>"));if kind=="teams"{assert!(html.contains("北京"));assert!(html.contains("background:rgb("));}if kind=="time"{assert!(html.contains("class=\"team\""));}}
    e.alliances=vec![vec![1,2],vec![3,4],vec![5,6],vec![7,8]];tournament_manager_rs::bracket::start(&mut e,&g,1).unwrap();let html=reports::pdf_html(&e,"bracket").unwrap();assert!(html.contains("<svg"));assert!(html.contains("1A / 2A"));assert!(!html.contains("北京"));
}
