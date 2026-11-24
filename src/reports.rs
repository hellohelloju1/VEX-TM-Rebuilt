use crate::model::*;
use base64::Engine;
fn esc(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn title(m: &Match) -> String {
    format!(
        "{}{}",
        match m.round {
            1 => "P",
            2 => "Q",
            15 => "F",
            _ => "E",
        },
        m.number
    )
}
fn org_key(org: &str)->String{org.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()}
fn organization_colors(e:&Event)->std::collections::BTreeMap<String,[f64;3]>{
    let palette=[[0.89,0.82,0.98],[0.80,0.89,0.99],[0.80,0.95,0.85],[1.0,0.88,0.74],[0.99,0.82,0.88],[0.77,0.94,0.95],[0.98,0.95,0.74],[0.91,0.87,0.80],[0.86,0.91,0.75],[0.86,0.84,0.98],[0.97,0.87,0.96],[0.80,0.90,0.88]];
    let keys=e.teams.iter().map(|t|org_key(&t.organization)).filter(|k|!k.is_empty()).collect::<std::collections::BTreeSet<_>>();
    let mut used=std::collections::BTreeSet::new();let mut result=std::collections::BTreeMap::new();
    for key in keys{let hash=key.as_bytes().iter().fold(14695981039346656037u64,|h,b|(h^(*b as u64)).wrapping_mul(1099511628211));let mut index=hash as usize%palette.len();
        if used.len()<palette.len(){while used.contains(&index){index=(index+1)%palette.len();}used.insert(index);}
        result.insert(key,palette[index]);
    }result
}
fn team_color(e:&Event,id:u32,colors:&std::collections::BTreeMap<String,[f64;3]>)->Option<[f64;3]>{e.teams.iter().find(|t|t.id==id).and_then(|t|colors.get(&org_key(&t.organization)).copied())}
fn color_css(color:[f64;3])->String{format!("#{:02x}{:02x}{:02x}",(color[0]*255.).round() as u8,(color[1]*255.).round() as u8,(color[2]*255.).round() as u8)}
fn colored_team(e:&Event,id:u32,colors:&std::collections::BTreeMap<String,[f64;3]>)->String{
    let number=esc(&e.team_number(id));match team_color(e,id,colors){Some(c)=>format!("<span class=\"org-team\" style=\"background:{}\">{number}</span>",color_css(c)),None=>number}
}
fn schedule_cell(e:&Event,m:&Match,colors:&std::collections::BTreeMap<String,[f64;3]>)->serde_json::Value{
    use serde_json::json;let mut runs=vec![];let mut plain=String::new();
    for (ai,a) in m.alliances.iter().enumerate(){if ai>0{runs.push(json!({"text":" vs "}));plain.push_str(" vs ");}for (ti,t) in a.teams.iter().enumerate(){if ti>0{runs.push(json!({"text":" / "}));plain.push_str(" / ");}let number=e.team_number(t.team);plain.push_str(&number);let mut run=json!({"text":number});if let Some(c)=team_color(e,t.team,colors){run["background"]=json!(c);}runs.push(run);}}
    json!({"text":plain,"runs":runs})
}
fn teams(e: &Event, m: &Match) -> String {
    let colors=organization_colors(e);
    m.alliances
        .iter()
        .map(|a| {
            a.teams
                .iter()
                .map(|t| colored_team(e,t.team,&colors))
                .collect::<Vec<_>>()
                .join(" / ")
        })
        .collect::<Vec<_>>()
        .join(" vs ")
}
pub fn schedule(e: &Event, by_team: bool) -> String {
    let font = base64::engine::general_purpose::STANDARD
        .encode(include_bytes!("../frontend/fonts/ReadexPro.ttf"));
    let mut html = format!(
        "<!doctype html><meta charset=utf-8><title>{} – Schedule</title><style>@font-face{{font-family:Readex;src:url(data:font/ttf;base64,{font})}}body{{font:14px Readex;color:#111;margin:28px}}h1{{font-size:25px}}h2{{break-after:avoid}}table{{border-collapse:collapse;width:100%;margin-bottom:24px}}th,td{{padding:7px;border-bottom:1px solid #ccc;text-align:left}}tr{{break-inside:avoid}}.org-team{{padding:2px 4px;color:#111;print-color-adjust:exact;-webkit-print-color-adjust:exact}}button{{font:18px Readex;padding:12px 22px}}@media print{{button{{display:none}}thead{{display:table-header-group}}}}</style><button onclick=\"window.print()\">Print schedule</button><h1>{}</h1><p>Schedule by {}</p>",
        esc(&e.name),
        esc(&e.name),
        if by_team { "team" } else { "time" }
    );
    let mut matches = e.matches.iter().collect::<Vec<_>>();
    matches.sort_by(|a, b| {
        a.planned
            .is_empty()
            .cmp(&b.planned.is_empty())
            .then(a.planned.cmp(&b.planned))
            .then(a.round.cmp(&b.round))
            .then(a.number.cmp(&b.number))
    });
    let header = "<table><thead><tr><th>Time<th>Match<th>Field<th>Teams</tr></thead><tbody>";
    if by_team {
        let mut ts = e.teams.iter().collect::<Vec<_>>();
        ts.sort_by(|a, b| a.number.cmp(&b.number));
        for t in ts {
            html += &format!(
                "<h2>Team {} — {}</h2>{header}",
                esc(&t.number),
                esc(&t.name)
            );
            for m in &matches {
                if m.alliances
                    .iter()
                    .any(|a| a.teams.iter().any(|s| s.team == t.id))
                {
                    html += &row(e, m)
                }
            }
            html += "</tbody></table>";
        }
    } else {
        html += header;
        for m in matches {
            html += &row(e, m)
        }
        html += "</tbody></table>"
    }
    let colors=organization_colors(e);
    if !colors.is_empty(){html+="<h2>Organizations</h2><table><tbody>";for (key,c) in &colors{let name=e.teams.iter().find(|t|org_key(&t.organization)==*key).map(|t|t.organization.trim()).unwrap_or(key);html+=&format!("<tr><td class=\"org-team\" style=\"background:{}\">{}</td></tr>",color_css(*c),esc(name));}html+="</tbody></table>";}
    html
}
fn row(e: &Event, m: &Match) -> String {
    format!(
        "<tr><td>{}<td>{}<td>{}<td>{}</tr>",
        esc(&m.planned),
        title(m),
        esc(&m.field),
        teams(e, m)
    )
}

pub fn pdf_data(e: &Event, kind: &str) -> anyhow::Result<serde_json::Value> {
    use serde_json::json;
    let team=|id:u32|e.teams.iter().find(|t|t.id==id).map(|t|format!("{} · {}",t.number,t.name)).unwrap_or_else(||"—".into());
    let section=|name:String,columns:Vec<&str>,widths:Vec<f64>,rows:Vec<Vec<String>>|json!({"name":name,"columns":columns,"widths":widths,"rows":rows});
    let mut sections=vec![];let mut bracket=vec![];
    let title=match kind {
        "time"|"team"=>{
            let mut ms=e.matches.iter().collect::<Vec<_>>();ms.sort_by_key(|m|(m.planned.is_empty(),&m.planned,m.round,m.number));
            let row=|m:&&Match|vec![m.planned.clone(),title(m),m.field.clone(),m.alliances.iter().map(|a|a.teams.iter().map(|s|e.team_number(s.team)).collect::<Vec<_>>().join(" / ")).collect::<Vec<_>>().join(" vs ")];
            if kind=="team"{for t in &e.teams{sections.push(section(team(t.id),vec!["Time","Match","Field","Teams"],vec![0.25,0.12,0.18,0.45],ms.iter().filter(|m|m.alliances.iter().any(|a|a.teams.iter().any(|s|s.team==t.id))).map(row).collect()));}}else{sections.push(section(String::new(),vec!["Time","Match","Field","Teams"],vec![0.25,0.12,0.18,0.45],ms.iter().map(row).collect()));}
            if kind=="team"{"Schedule by team"}else{"Schedule by time"}
        }
        "teams"=>{sections.push(section(String::new(),vec!["Number","Team name","Organization","Age group","City"],vec![0.12,0.27,0.28,0.15,0.18],e.teams.iter().map(|t|vec![t.number.clone(),t.name.clone(),t.organization.clone(),t.age_group.clone(),t.city.clone()]).collect()));"Team list"}
        "rankings"=>{
            let g=crate::scoring::Game::all().into_iter().find(|g|g.id()==e.game).ok_or_else(||anyhow::anyhow!("Unknown game"))?;
            let rows=crate::scoring::rankings(e,&g)?.iter().enumerate().map(|(i,r)|vec![(i+1).to_string(),team(r.team),format!("{:0.2}",if g.cooperative(){r.average_score}else{r.wp}),format!("{:0.2}",r.ap),format!("{:0.2}",r.sp),format!("{}-{}-{}",r.wins,r.losses,r.ties),r.played.to_string()]).collect();
            sections.push(section(String::new(),vec!["Rank","Team",if g.cooperative(){"Average"}else{"WP"},"AP","SP","W-L-T","Played"],vec![0.06,0.4,0.12,0.1,0.08,0.14,0.1],rows));
            if e.matches.iter().any(|m|m.round==2&&m.state!=MatchState::Submitted)||!e.matches.iter().any(|m|m.round==2){"Qualification rankings (provisional)"}else{"Final qualification rankings"}
        }
        "skills"=>{
            let mut rows=e.teams.iter().map(|t|{let drive=e.skills.iter().filter(|s|s.team==t.id&&!s.programming).map(|s|s.score).max().unwrap_or(0);let auto=e.skills.iter().filter(|s|s.team==t.id&&s.programming).map(|s|s.score).max().unwrap_or(0);(t.id,drive,auto,drive+auto)}).collect::<Vec<_>>();rows.sort_by_key(|r|std::cmp::Reverse(r.3));
            sections.push(section(String::new(),vec!["Rank","Team","Driving","Autonomous","Total"],vec![0.07,0.45,0.16,0.16,0.16],rows.iter().enumerate().map(|(i,r)|vec![(i+1).to_string(),team(r.0),r.1.to_string(),r.2.to_string(),r.3.to_string()]).collect()));"Skills rankings"
        }
        "bracket"=>{
            let order=[9,8,7,6,3,4,5];if let Some(first)=order.iter().position(|r|e.bracket.iter().any(|s|s.round==*r)){
                let base=e.bracket.iter().filter(|s|s.round==order[first]).count();for (col,round) in order[first..].iter().enumerate(){let count=(base+(1<<col)-1)/(1<<col);let actual=e.bracket.iter().filter(|s|s.round==*round).collect::<Vec<_>>();let series=(0..count).map(|i|if let Some(s)=actual.get(i){json!({"teams":s.teams.iter().map(|a|if a.is_empty(){"BYE".into()}else{a.iter().map(|id|e.team_number(*id)).collect::<Vec<_>>().join(" / ")}).collect::<Vec<String>>(),"wins":s.wins,"winner":s.winner})}else{json!({"teams":["TBD","TBD"],"wins":["",""],"winner":null})}).collect::<Vec<_>>();bracket.push(json!({"name":match round{5=>"Final".into(),4=>"Semifinals".into(),3=>"Quarterfinals".into(),_=>format!("Round of {}",count*2)},"series":series}));}
            }else{sections.push(section(String::new(),vec!["Match","Teams","State"],vec![0.15,0.65,0.2],e.matches.iter().filter(|m|m.round>2).map(|m|vec![title(m),m.alliances.iter().map(|a|a.teams.iter().map(|s|e.team_number(s.team)).collect::<Vec<_>>().join(" / ")).collect::<Vec<_>>().join(" vs "),format!("{:?}",m.state)]).collect()));}
            "Elimination bracket / finals"
        }
        "results"=>{
            let g=crate::scoring::Game::all().into_iter().find(|g|g.id()==e.game).ok_or_else(||anyhow::anyhow!("Unknown game"))?;
            let rows=e.matches.iter().map(|m|{let score=if matches!(m.state,MatchState::Scored|MatchState::Submitted){g.score(m).ok().map(|v|v["alliances"].as_array().unwrap_or(&vec![]).iter().map(|a|format!("{}",a["score"].as_f64().unwrap_or(0.))).collect::<Vec<_>>().join(" - ")).unwrap_or_default()}else{String::new()};vec![title(m),m.planned.clone(),m.field.clone(),m.alliances.iter().map(|a|a.teams.iter().map(|s|e.team_number(s.team)).collect::<Vec<_>>().join(" / ")).collect::<Vec<_>>().join(" vs "),score,format!("{:?}",m.state)]}).collect();
            sections.push(section(String::new(),vec!["Match","Time","Field","Teams","Score","State"],vec![0.08,0.18,0.12,0.36,0.12,0.14],rows));"Match results"
        }
        "awards"=>{sections.push(section(String::new(),vec!["Award","Team / recipient"],vec![0.5,0.5],e.awards.iter().map(|a|vec![a.name.clone(),if a.team>0{team(a.team)}else{a.recipient.clone()}]).collect()));"Awards"}
        _=>anyhow::bail!("Unknown PDF report"),
    };
    if matches!(kind,"time"|"team"|"teams"){
        let colors=organization_colors(e);
        if kind=="teams"{sections[0]["row_colors"]=json!(e.teams.iter().map(|t|team_color(e,t.id,&colors)).collect::<Vec<_>>());}
        else{
            let mut ms=e.matches.iter().collect::<Vec<_>>();ms.sort_by_key(|m|(m.planned.is_empty(),&m.planned,m.round,m.number));
            if kind=="time"{for (i,m) in ms.iter().enumerate(){sections[0]["rows"][i][3]=schedule_cell(e,m,&colors);}}
            else{for (section,t) in sections.iter_mut().zip(&e.teams){for (i,m) in ms.iter().filter(|m|m.alliances.iter().any(|a|a.teams.iter().any(|s|s.team==t.id))).enumerate(){section["rows"][i][3]=schedule_cell(e,m,&colors);}}}
        }
        if !colors.is_empty(){let mut legend=section("Organizations".into(),vec!["Organization"],vec![1.0],colors.keys().map(|key|vec![e.teams.iter().find(|t|org_key(&t.organization)==*key).map(|t|t.organization.trim().to_owned()).unwrap_or_else(||key.clone())]).collect());legend["row_colors"]=json!(colors.values().collect::<Vec<_>>());sections.push(legend);}
    }
    Ok(json!({"event":e.name,"title":title,"landscape":matches!(kind,"rankings"|"results"|"teams"),"sections":sections,"bracket":bracket}))
}
pub fn export_pdf(path: &std::path::Path,e: &Event,kind:&str)->anyhow::Result<()> {
    use anyhow::Context;use std::io::Write;use std::process::{Command,Stdio};
    let resources=crate::platform::resources()?;
    #[cfg(target_os="windows")]
    let (helper,args,data)=("report-pdf.exe",vec![if pdf_data(e,kind)?["landscape"]==true{"landscape"}else{"portrait"}.to_owned()],pdf_html(e,kind)?.into_bytes());
    #[cfg(not(target_os="windows"))]
    let (helper,args,data)=("report-pdf",vec![resources.join("ReadexPro.ttf").to_string_lossy().into_owned()],serde_json::to_vec(&pdf_data(e,kind)?)?);
    let mut command=Command::new(resources.join(helper));command.arg(path).args(args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
    #[cfg(target_os="windows")]
    {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    let mut child=command.spawn().context("PDF exporter unavailable")?;
    child.stdin.take().context("PDF input unavailable")?.write_all(&data)?;
    let result=child.wait_with_output()?;if !result.status.success(){anyhow::bail!("PDF export failed: {}",String::from_utf8_lossy(&result.stderr));}Ok(())
}

/// Shared report data rendered with the app font for the Windows WebView2 PDF helper.
pub fn pdf_html(e:&Event,kind:&str)->anyhow::Result<String>{
    use base64::Engine;
    let data=pdf_data(e,kind)?;
    let escape=esc;
    let esc=|v:&serde_json::Value|escape(v.as_str().unwrap_or(""));
    let color=|v:&serde_json::Value|->String{v.as_array().filter(|a|a.len()==3).map(|a|format!("rgb({},{},{})",(a[0].as_f64().unwrap_or(1.)*255.).round(),(a[1].as_f64().unwrap_or(1.)*255.).round(),(a[2].as_f64().unwrap_or(1.)*255.).round())).unwrap_or_else(||"transparent".into())};
    let font=base64::engine::general_purpose::STANDARD.encode(include_bytes!("../frontend/fonts/ReadexPro.ttf"));
    let mut html=format!("<!doctype html><meta charset=\"utf-8\"><title>{}</title><style>@font-face{{font-family:AppFont;src:url(data:font/ttf;base64,{font})}}@page{{size:A4 {};margin:14mm}}*{{box-sizing:border-box;print-color-adjust:exact;-webkit-print-color-adjust:exact}}body{{font:10pt AppFont,sans-serif;color:#17243a}}h1{{font-size:22pt;color:#812bbb;border-bottom:3px solid #812bbb;padding-bottom:8px}}h2{{font-size:14pt;color:#812bbb;break-after:avoid}}table{{border-collapse:collapse;width:100%;margin:8px 0 20px;table-layout:fixed}}thead{{display:table-header-group}}th,td{{border:1px solid #91839e;padding:7px;overflow-wrap:anywhere;vertical-align:top}}th{{background:#812bbb;color:white;text-align:left}}tr{{break-inside:avoid}}tbody tr:nth-child(even){{background:#f2edf6}}.team{{padding:2px 3px;display:inline-block}}svg{{width:100%;height:auto}}svg text{{font:12px AppFont,sans-serif;fill:#17243a}}svg rect{{fill:#f6f0fa;stroke:#812bbb}}svg path{{fill:none;stroke:#91839e}}</style><div>{}</div><h1>{}</h1>",esc(&data["title"]),if data["landscape"]==true{"landscape"}else{"portrait"},escape(&e.name),esc(&data["title"]));
    if let Some(sections)=data["sections"].as_array(){for section in sections{html.push_str(&format!("<h2>{}</h2><table><colgroup>",esc(&section["name"])));let widths=section["widths"].as_array().unwrap();let sum:f64=widths.iter().map(|v|v.as_f64().unwrap_or(1.)).sum();for w in widths{html.push_str(&format!("<col style=\"width:{}%\">",w.as_f64().unwrap_or(1.)/sum*100.));}html.push_str("</colgroup><thead><tr>");for column in section["columns"].as_array().unwrap(){html.push_str(&format!("<th>{}</th>",esc(column)));}html.push_str("</tr></thead><tbody>");for(i,row)in section["rows"].as_array().unwrap().iter().enumerate(){let tint=color(&section["row_colors"][i]);html.push_str(&format!("<tr style=\"background:{tint}\">"));for cell in row.as_array().unwrap(){html.push_str("<td>");if let Some(runs)=cell["runs"].as_array(){for run in runs{html.push_str(&format!("<span class=\"team\" style=\"background:{}\">{}</span>",color(&run["background"]),esc(&run["text"])));}}else{html.push_str(&esc(cell));}html.push_str("</td>");}html.push_str("</tr>");}html.push_str("</tbody></table>");}}
    if let Some(cols)=data["bracket"].as_array().filter(|a|!a.is_empty()){
        let height=(cols[0]["series"].as_array().unwrap().len()*90+40).max(180);let width=cols.len()*230;
        html.push_str(&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\">"));
        for(c,col)in cols.iter().enumerate(){let series=col["series"].as_array().unwrap();let step=(height-40)as f64/series.len()as f64;html.push_str(&format!("<text x=\"{}\" y=\"18\">{}</text>",c*230+8,esc(&col["name"])));for(i,item)in series.iter().enumerate(){let x=c*230+8;let y=40.+step*(i as f64+0.5)-25.;html.push_str(&format!("<rect x=\"{x}\" y=\"{y}\" width=\"200\" height=\"50\"/>"));for(a,team)in item["teams"].as_array().unwrap().iter().enumerate(){html.push_str(&format!("<text x=\"{}\" y=\"{}\">{}</text>",x+8,y+18.+a as f64*22.,esc(team)));}if c+1<cols.len(){let next_count=cols[c+1]["series"].as_array().unwrap().len();let next_step=(height-40)as f64/next_count as f64;let target=40.+next_step*((i/2)as f64+0.5);html.push_str(&format!("<path d=\"M {} {} H {} V {target} H {}\"/>",x+200,y+25.,x+211,x+230));}}}html.push_str("</svg>");
    }
    Ok(html)
}
