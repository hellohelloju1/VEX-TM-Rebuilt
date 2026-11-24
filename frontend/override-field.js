export const overrideRegions=[
 {id:'red_quad_1',label:'Red 1',color:'red',points:'240,166 295,166 295,99 525,332 468,390 525,449 295,680 295,614 240,614'},
 {id:'red_quad_2',label:'Red 2',color:'red',points:'295,680 525,449 584,507 642,449 872,680 872,732 295,732'},
 {id:'blue_quad_1',label:'Blue 1',color:'blue',points:'872,99 872,166 927,166 927,614 872,614 872,680 642,449 700,390 642,332'},
 {id:'blue_quad_2',label:'Blue 2',color:'blue',points:'295,49 872,49 872,99 642,332 584,273 525,332 295,99'},
 {id:'center',label:'Center',color:'yellow',points:'584,278 696,390 584,502 472,390'}
];
export function overrideRegion(key){return overrideRegions.find(r=>key.startsWith(r.id+'_'));}
export function overrideFieldView(panels,selected='red_quad_1'){
 return `<section class="override-scoring"><h3>Field scoring</h3><div class="override-field-layout"><div class="override-field-guide"><div class="override-map"><img src="override-quadrants.png" alt="Override quadrants: Blue 2 top, Red 1 left, Blue 1 right, Red 2 bottom, Center diamond"><svg viewBox="0 0 1166 781" aria-label="Choose scoring region">${overrideRegions.map(r=>`<polygon points="${r.points}" class="override-hit ${r.id===selected?'selected':''}" role="button" tabindex="0" data-override-region="${r.id}" aria-label="Score ${r.label}" aria-pressed="${r.id===selected}"/>`).join('')}</svg></div><div class="override-region-tabs" aria-label="Field regions">${overrideRegions.map(r=>`<button type="button" data-override-region="${r.id}" aria-pressed="${r.id===selected}" class="region-${r.color} ${r.id===selected?'selected':''}">${r.label}</button>`).join('')}</div><details class="override-reference"><summary>Field reference</summary><img src="override-field.png" alt="Override field showing pin locations, quadrant boundaries and center diamond"></details></div><div class="override-region-controls">${overrideRegions.map(r=>`<section class="override-region-panel region-${r.color}" data-override-panel="${r.id}" ${r.id===selected?'':'hidden'}><h3>${r.label}</h3><div class="score-grid">${(panels[r.id]||[]).join('')}</div></section>`).join('')}</div></div></section>`;
}
export function selectOverrideRegion(root,id){
 if(!overrideRegions.some(r=>r.id===id))return;
 root.querySelectorAll('[data-override-region]').forEach(el=>{const selected=el.dataset.overrideRegion===id;el.classList.toggle('selected',selected);el.setAttribute('aria-pressed',String(selected));});
 root.querySelectorAll('[data-override-panel]').forEach(el=>{el.hidden=el.dataset.overridePanel!==id;});
}
