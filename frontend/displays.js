import {escapeHtml as h,clock} from './planner.js';
export async function startDisplay(id,fetchState){
 const root=document.querySelector('#app');document.body.className='display-body';let current,signature='',offset=0,last=performance.now(),loopHeight=0,audienceOffset=0,audienceHeight=0,audienceSignature='';
 const team=(e,id)=>e.teams.find(t=>t.id===id)?.number||'—';
 function image(s){let str='';const a=new Uint8Array(s.image);for(let i=0;i<a.length;i+=8192)str+=String.fromCharCode(...a.subarray(i,i+8192));return `data:${a[0]===137?'image/png':'image/jpeg'};base64,${btoa(str)}`;}
 function update(s){
  render(s);
  const d=s.event.config.displays.find(d=>d.id===id),el=root.querySelector('[data-scheduled-start]');
  if(el){const text=scheduledStartLabel(s,d);el.textContent=text;el.hidden=!text;el.closest('.display-head')?.classList.toggle('has-scheduled-start',!!text);}
  fitDisplay(root);
 }
 function render(s){current=s;const e=s.event,d=e.config.displays.find(d=>d.id===id)||e.config.displays.find(d=>d.kind==='Pit')||{kind:'Pit',screen:'Rankings'},iq=e.game.startsWith('VIQC');const mode=s.field_control?.audience||'None',rankingView=d.kind==='Pit'||(d.kind==='Audience'&&['Rankings','Skills Rankings'].includes(mode)),skills=d.kind==='Pit'?d.screen==='Skills':mode==='Skills Rankings';const sponsors=e.config.sponsors.filter(s=>s.image?.length),head=`<header class="display-head"><div><div class="display-event" title="${h(e.name)}">${h(e.name)}</div><h1>${rankingView?(skills?'ROBOT SKILLS':'RANKINGS'):h(d.kind==='Audience'?mode:(d.kind==='Field'?'':(d.name||d.kind)))}</h1>${['Audience','Field'].includes(d.kind)?'<div class="display-scheduled" data-scheduled-start hidden></div>':''}</div><div><img class="display-logo" src="${iq?'iq':'v5'}-display.png" alt="${iq?'VEX IQ':'VEX V5'}">${sponsors.length?`<img class="display-sponsor" src="${image(sponsors[Math.floor(Date.now()/15000)%sponsors.length])}" alt="Sponsor">`:''}</div></header>`;
 if(rankingView){
  audienceSignature='';audienceHeight=0;
  const content=rankingsContent(s,skills),sig=JSON.stringify([head,content]);
  if(sig===signature)return;signature=sig;
  root.innerHTML=`<div class="display-root">${head}${content}</div>`;
  const unit=root.querySelector('#pit-unit'),container=root.querySelector('.pit-scroll'),belt=root.querySelector('.pit-stream');
  loopHeight=unit.getBoundingClientRect().height;
  if(loopHeight>0){const copies=Math.ceil(container.clientHeight/loopHeight)+2;for(let i=1;i<copies;i++)belt.append(unit.cloneNode(true));offset%=loopHeight;belt.style.transform=`translateY(${-offset}px)`;}
  return;
 }
 signature='';loopHeight=0;
 if(d.kind==='Audience'&&!['In-Match','Up Next','Saved Match Results'].includes(mode)){
   const content=audienceContent(s,mode,iq,{width:window.innerWidth,height:window.innerHeight}),sig=JSON.stringify([mode,head,content]);
   if(sig===audienceSignature)return;audienceSignature=sig;
   root.innerHTML=mode==='None'?'<div class="display-root"></div>':`<div class="display-root">${head}${content}</div>`;
   const table=root.querySelector('.table-wrap table');audienceHeight=0;
   if(table){const wrap=table.parentElement,header=table.cloneNode(false);header.append(table.tHead.cloneNode(true));table.tHead.remove();wrap.classList.add('audience-scroll');wrap.innerHTML='';wrap.append(header);const viewport=document.createElement('div');viewport.className='audience-scroll-viewport';const belt=document.createElement('div');belt.className='audience-stream';belt.append(table);viewport.append(belt);wrap.append(viewport);audienceHeight=table.getBoundingClientRect().height;if(audienceHeight){for(let i=0,n=Math.ceil(viewport.clientHeight/audienceHeight)+1;i<n;i++)belt.append(table.cloneNode(true));audienceOffset%=audienceHeight;belt.style.transform=`translateY(${-audienceOffset}px)`;}}
   return;
 }
 audienceSignature='';
 {const saved=d.kind==='Audience'&&mode==='Saved Match Results';const active=saved?s.field_control?.saved:e.matches.find(m=>m.id===s.active&&(!d.field||m.field===d.field)),scores=(saved?s.field_control?.saved_score:s.scores)?.alliances||[];root.innerHTML=`<div class="display-root">${head}${d.kind==='Field'||mode==='In-Match'?`<div class="display-clock ${s.timer.phase==='Countdown'?'prestart-number':''}">${s.timer.phase==='Countdown'?s.timer.seconds:clock(s.timer.seconds)}</div>`:''}<div class="match-status">${h(s.timer.phase)}${active?` · ${active.round===1?'Practice':'Match'} ${active.number}`:' · Awaiting match'}</div><div class="display-teams">${active?active.alliances.map((a,i)=>`<div class="${i?'blue':'red'}"><div class="display-roster">${a.teams.map(t=>teamIdentity(e,t.team)).join('')}</div><div class="display-scores">${scorePreviewVisible(s,saved)?(scores[i]?.score??''):''}</div></div>`).join(''):`<div>${h(e.announcements||'Welcome')}</div>`}</div>${d.kind==='Audience'?`<section class="audience-upcoming"><h3>Upcoming matches</h3>${upcomingMatches(s).slice(0,3).map(m=>`<div><b>${m.round===1?'P':m.round===2?'Q':'E'}${m.number}</b> · ${m.alliances.map(a=>a.teams.map(t=>h(team(e,t.team))).join(' / ')).join(' vs ')} · ${h(m.field)}</div>`).join('')}</section>`:''}</div>`;}
 }
 function animate(now){const dt=Math.min((now-last)/1000,.1);last=now;const belt=document.querySelector('.pit-stream');if(belt&&loopHeight){offset=(offset+dt*27)%loopHeight;belt.style.transform=`translateY(${-offset}px)`;}const audienceBelt=document.querySelector('.audience-stream');if(audienceBelt&&audienceHeight){audienceOffset=(audienceOffset+dt*27)%audienceHeight;audienceBelt.style.transform=`translateY(${-audienceOffset}px)`;}requestAnimationFrame(animate);}
 await fetchState().then(update);document.fonts?.ready.then(()=>{signature='';audienceSignature='';if(current)update(current);});requestAnimationFrame(animate);let inFlight=false;setInterval(async()=>{if(inFlight)return;inFlight=true;try{update(await fetchState());}finally{inFlight=false;}},500);window.addEventListener('resize',()=>{signature='';audienceSignature='';if(current)update(current);});
}
function formatTime(t){const m=/(\d{2}):(\d{2})/.exec(t);if(!m)return t||'—';let h=+m[1];return `${h%12||12}:${m[2]} ${h>=12?'PM':'AM'}`;}

export function audienceContent(s,mode,iq,viewport={width:1920,height:1080}){
 const e=s.event,team=id=>e.teams.find(t=>t.id===id)?.number||'—';
 const table=(headers,rows)=>`<div class="table-wrap"><table><thead><tr>${headers.map(x=>`<th>${h(x)}</th>`).join('')}</tr></thead><tbody>${rows.map(r=>`<tr>${r.map(x=>`<td>${h(x)}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`;
 if(mode==='Logo')return `<div class="audience-logo-stage"><img src="${iq?'iq':'v5'}-display.png" alt="${iq?'VEX IQ':'VEX V5'}"></div>`;
 if(mode==='Rankings')return rankingsContent(s,false);
 if(mode==='Schedule')return table(['Match','Time','Field','Teams'],e.matches.filter(m=>m.state!=='Submitted').map(m=>[`${m.round===1?'P':m.round===2?'Q':'E'}${m.number}`,m.planned,m.field,m.alliances.map(a=>a.teams.map(t=>team(t.team)).join(' / ')).join(' vs ')]));
 if(mode==='Inspection')return table(['Team','Name','Inspection'],e.teams.map(t=>[t.number,t.name,t.inspected?'Passed':'Pending']));
 if(mode==='Alliance Selection')return allianceSelectionContent(e,viewport);
 if(mode==='Elim Bracket')return bracketContent(e);
 if(mode==='Skills Rankings')return rankingsContent(s,true);
 if(mode==='Award Slides'){const a=e.awards[Math.floor(Date.now()/8000)%Math.max(e.awards.length,1)];return `<div class="award-slide"><h1>${h(a?.name||'Awards')}</h1><p class="award-recipient">${h(a?.team?team(a.team):(a?.recipient||''))}</p><p class="award-description">${h(a?.description||'')}</p></div>`;}
 return '';
}

export function upcomingMatches(s){
 const index=s.event.matches.findIndex(m=>m.id===s.active);
 return s.event.matches.slice(index<0?0:index+1).filter(m=>m.state!=='Submitted');
}
export function bracketContent(e){
 if(!e.bracket.length)return '<div class="empty">No elimination bracket</div>';
 const order=[9,8,7,6,3,4,5],first=order.findIndex(r=>e.bracket.some(s=>s.round===r)),rounds=order.slice(first),base=e.bracket.filter(s=>s.round===rounds[0]).length;
 const width=rounds.length*290,height=Math.max(150,base*110),team=id=>e.teams.find(t=>t.id===id)?.number||'—';
 let lines='',boxes='';
 rounds.forEach((round,c)=>{const count=Math.max(1,Math.ceil(base/2**c)),series=e.bracket.filter(s=>s.round===round);for(let i=0;i<count;i++){
 const x=c*290+10,y=(i+.5)*height/count,s=series[i];
 if(c){for(const from of [2*i,2*i+1]){const previous=Math.max(1,Math.ceil(base/2**(c-1)));if(from<previous){const py=(from+.5)*height/previous;lines+=`<path d="M ${x-40} ${py} H ${x-20} V ${y} H ${x}"/>`;}}}
 const title=round===5?'Final':round===4?'Semifinal':round===3?'Quarterfinal':`Round of ${count*2}`;
 boxes+=`<g><text x="${x}" y="${y-43}" class="bracket-label">${h(title)} ${i+1}</text><rect x="${x}" y="${y-34}" width="240" height="68" rx="0"/>${[0,1].map(a=>{const ids=s?.teams[a]||[],name=ids.length?ids.map(team).join(' / '):s?'BYE':'TBD';return `<text x="${x+12}" y="${y-10+a*32}" class="${s?.winner===a?'bracket-winner':''}">${h(name)}</text><text x="${x+216}" y="${y-10+a*32}">${s?.wins[a]??''}</text>`;}).join('')}</g>`;
 }});
 return `<div class="audience-bracket"><svg role="img" aria-label="Elimination bracket" style="min-width:${Math.ceil(width*0.65)}px;min-height:${Math.ceil((height+40)*0.65)}px" viewBox="0 -20 ${width} ${height+40}" preserveAspectRatio="xMidYMid meet"><g class="bracket-lines">${lines}</g>${boxes}</svg></div>`;
}

export function scheduledStartLabel(s,display){
 if(!display||!['Audience','Field'].includes(display.kind)||s.timer?.phase!=='Idle')return '';
 const match=s.event.matches.find(m=>m.id===s.active);
 if(!match||match.state==='Submitted'||(display.kind==='Field'&&display.field&&display.field!==match.field))return '';
 if(!Number.isFinite(s.scheduled_start_ms)||!Number.isFinite(s.server_time_ms))return '';
 const seconds=Math.max(0,Math.ceil((s.scheduled_start_ms-s.server_time_ms)/1000));
 return `Scheduled start in: ${clock(seconds)}`;
}

export function teamIdentity(e,id){
 const t=e.teams.find(t=>t.id===id);
 return `<span class="display-team" title="${h(t?`${t.number} · ${t.name}`:'—')}"><strong>${h(t?.number||'—')}</strong>${t?.name?`<span class="display-team-name">${h(t.name)}</span>`:''}</span>`;
}
function sponsorSource(s){let str='';const bytes=new Uint8Array(s.image);for(let i=0;i<bytes.length;i+=8192)str+=String.fromCharCode(...bytes.subarray(i,i+8192));return `data:${bytes[0]===137?'image/png':'image/jpeg'};base64,${btoa(str)}`;}
export function skillsRankings(e){
 return e.teams.map(t=>{const attempts=e.skills.filter(s=>s.team===t.id),drive=Math.max(0,...attempts.filter(s=>!s.programming).map(s=>s.score)),auto=Math.max(0,...attempts.filter(s=>s.programming).map(s=>s.score));return{team:t.id,drive,auto,total:drive+auto};}).sort((a,b)=>b.total-a.total);
}
export function rankingsContent(s,skills=false){
 const recent=(s.recent_quals||[]).slice(0,2);const e=s.event,iq=e.game.startsWith('VIQC'),rows=skills?skillsRankings(e):(s.rankings||[]),sponsors=(e.config.sponsors||[]).filter(s=>s.image?.length);
 let stream='';rows.forEach((r,i)=>{
  if(sponsors.length&&i%12===0)stream+=`<div class="pit-sponsor"><img src="${sponsorSource(sponsors[Math.floor(i/12)%sponsors.length])}" alt="Sponsor"></div>`;
  const team=e.teams.find(t=>t.id===r.team);
  stream+=`<div class="pit-row ${i%2?'shade':''}"><b>${i+1}</b><b>${h(team?.number||'—')}</b><span class="ranking-team-name">${h(team?.name||'')}</span>${skills?`<span>${r.drive}</span><span>${r.auto}</span><b>${r.total}</b>`:`<span class="rank-wp">${(iq?r.average_score:r.wp).toFixed(2)}</span><span class="rank-ap">${r.ap.toFixed(2)}</span><span class="rank-sp">${r.sp.toFixed(2)}</span><span class="rank-record">${r.wins??0}-${r.losses??0}-${r.ties??0}</span>`}</div>`;
 });
 if(!rows.length)stream='<div class="pit-row"><span>—</span><span></span><span>Awaiting teams</span></div>';
 return `<div class="pit-columns ${skills?'pit-skills':''}"><section class="pit-left"><div class="pit-head"><span>RANK</span><span>TEAM</span><span>NAME</span>${skills?'<span>DRIVING</span><span>AUTO</span><span>TOTAL</span>':`<span class="rank-wp">${iq?'AVG':'WP'}</span><span class="rank-ap">AP</span><span class="rank-sp">SP</span><span class="rank-record">WLT</span>`}</div><div class="pit-scroll"><div class="pit-stream"><div id="pit-unit">${stream}</div></div></div></section><section class="pit-right"><h3>MATCH SCHEDULE &amp; RESULTS</h3>${recent.map(result=>completedQualCard(e,result)).join('')}${upcomingMatches(s).slice(0,6-recent.length).map(m=>`<div class="upcoming"><b>${m.round===1?'P':m.round===2?'Q':'E'}${m.number}</b><div class="teams">${m.alliances.map((a,i)=>a.teams.map(t=>`<span class="${iq||i?'blue':'red'}">${h(e.teams.find(x=>x.id===t.team)?.number||'—')}</span>`).join('')).join('')}</div><div class="place"><strong title="${h(m.field)}">${h(m.field)}</strong><time>${h(formatTime(m.planned))}</time></div></div>`).join('')}</section></div>`;
}

export function scorePreviewVisible(s,saved=false){
 return (saved&&s.field_control?.saved?.state==='Submitted')||s.field_control?.show_score_preview!==false;
}

export function allianceSelectionContent(e,{width=1920,height=1080,now=Date.now()}={}){
 const columns=Math.max(1,Math.min(8,Math.floor((width-24)/210))),rows=Math.max(1,Math.floor(Math.max(100,height-Math.max(90,height*.22))/120)),capacity=columns*rows,pages=Math.max(1,Math.ceil(e.alliances.length/capacity)),page=Math.floor(now/8000)%pages,start=page*capacity,alliances=e.alliances.slice(start,start+capacity),visibleColumns=Math.min(columns,Math.max(1,alliances.length));
 return `<div class="alliance-selection-stage"><div class="alliance-selection-cards" style="--alliance-columns:${visibleColumns}">${alliances.map((a,i)=>`<section class="alliance-pair"><h2>Seed ${start+i+1}</h2><div>${a.map(id=>teamIdentity(e,id)).join('')}</div></section>`).join('')||'<div class="empty">Awaiting alliance selection</div>'}</div>${pages>1?`<div class="alliance-page">${page+1} / ${pages}</div>`:''}</div>`;
}
export function fitDisplay(root){
 const clock=root.querySelector('.display-clock');
 if(clock){clock.style.fontSize='';const rect=clock.getBoundingClientRect();if(rect.width>0){const desired=parseFloat(getComputedStyle(clock).fontSize);const max=Math.min(Math.max(14,rect.height*.86),rect.width/Math.max(1,clock.textContent.length*.7));clock.style.fontSize=`${Math.max(12,Math.min(desired,max))}px`;}}
}

export function completedQualCard(e,result){
 const m=result.match,iq=e.game.startsWith('VIQC'),scores=result.score?.alliances||[];
 return `<div class="upcoming completed-qual"><div class="completed-label"><b>Q${m.number}</b><small>FINAL</small></div><div class="teams">${m.alliances.map((a,i)=>a.teams.map(t=>`<span class="${iq||i?'blue':'red'}">${h(e.teams.find(x=>x.id===t.team)?.number||'—')}</span>`).join('')).join('')}</div><div class="result-score">${iq?`<strong class="blue">${h(scores[0]?.score??'—')}</strong>`:scores.slice(0,2).map((a,i)=>`<strong class="${i?'blue':'red'}">${h(a.score??'—')}</strong>`).join('')}</div></div>`;
}
