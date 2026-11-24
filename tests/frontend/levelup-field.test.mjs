import {test} from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import {Window} from 'happy-dom';
import {levelUpRegions,levelUpRegion,levelUpFieldView} from '../../frontend/levelup-field.js';
test('Level Up diagrams group all nine goal inputs in teamwork and skills',async()=>{
 const game=JSON.parse(fs.readFileSync(new URL('../../analysis/games.json',import.meta.url))).find(g=>g.id==='VIQC_2026_LevelUp');
 for(const skills of [false,true]){const panels={};for(const c of game.controls.filter(c=>c.name.startsWith('skills_')===skills)){const key=c.name.replace(/^skills_a1_/,'');const region=levelUpRegion(key);if(region)(panels[region]??=[]).push(`<input data-score="${key}" value="3">`);}
 const w=new Window();w.document.body.innerHTML=levelUpFieldView(panels);assert.equal(w.document.querySelectorAll('.levelup-goal').length,3);assert.equal(w.document.querySelectorAll('svg image').length,3);assert.equal(w.document.querySelectorAll('input').length,9);for(const r of levelUpRegions)assert.ok(panels[r.id].length);assert.equal(w.document.querySelector('[data-score="goal_level_4"]').value,'3');await w.happyDOM.close();}
 assert.equal(levelUpRegion('stoptime'),null);assert.equal(levelUpRegion('skills_stop_time'),null);
});
