export const levelUpRegions=[
 {id:'red',label:'Red goals · Floor / L1 / L2 / L3',crop:'78 0 195 205',color:'red'},
 {id:'blue',label:'Blue goals · Floor / L1 / L2 / L3',crop:'725 455 195 190',color:'blue'},
 {id:'level4',label:'Level 4 goals',crop:'345 230 305 185',color:'yellow'}
];
export function levelUpRegion(key){return key.startsWith('red_goal_')?'red':key.startsWith('blue_goal_')?'blue':key==='goal_level_4'?'level4':null;}
export function levelUpFieldView(panels){
 return `<section class="levelup-scoring"><div class="levelup-goals">${levelUpRegions.map(r=>`<section class="levelup-goal region-${r.color}"><h3>${r.label}</h3><svg class="levelup-crop" viewBox="0 0 ${r.crop.split(' ').slice(2).join(' ')}" role="img" aria-label="${r.label} field diagram"><defs><clipPath id="levelup-${r.id}-clip"><rect x="0" y="0" width="${r.crop.split(' ')[2]}" height="${r.crop.split(' ')[3]}"/></clipPath></defs><g clip-path="url(#levelup-${r.id}-clip)"><image href="levelup-field.png" x="-${r.crop.split(' ')[0]}" y="-${r.crop.split(' ')[1]}" width="998" height="653"/></g></svg><div class="score-grid">${(panels[r.id]||[]).join('')}</div></section>`).join('')}</div><details class="override-reference"><summary>Full field reference</summary><img src="levelup-field.png" alt="Level Up field with corner goals L1, L2, L3 and central L4 goals"></details></section>`;
}
