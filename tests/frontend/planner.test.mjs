import {test} from 'node:test';import assert from 'node:assert/strict';
import {capacity,timeSeconds,balancedEnd,planStats,escapeHtml} from '../../frontend/planner.js';
test('capacity respects complete cycles and simultaneous fields',()=>{assert.equal(capacity({start:'09:00',end:'09:35',cycle_seconds:360},2),10);assert.equal(capacity({start:'10:00',end:'09:00',cycle_seconds:360},1),0);});
test('balanced day duration reserves complete counted rounds',()=>{assert.equal(balancedEnd('09:00',24,4,1,1,360),'09:36');assert.equal(balancedEnd('10:00',24,4,1,4,360),'12:24');assert.equal(balancedEnd('10:00',24,4,2,4,360),'11:12');assert.throws(()=>balancedEnd('23:00',24,4,1,4,360));});
test('invalid times do not silently create a plan',()=>{assert.throws(()=>timeSeconds('25:00'));assert.throws(()=>timeSeconds('12:60'));assert.throws(()=>balancedEnd('09:00',0,4,1,1,360));});
test('summary separates practice and qualification capacity',()=>{const e={teams:Array(24),config:{field_sets:[{fields:['Field 1']}],schedule:{blocks:[{start:'09:00',end:'09:36',cycle_seconds:360,round:1},{start:'10:00',end:'12:24',cycle_seconds:360,round:2}]}}};assert.deepEqual(planStats(e,4),{fields:1,practice:6,qualifications:24,practicePerTeam:1,qualificationPerTeam:4});});
test('event names and team text are escaped',()=>assert.equal(escapeHtml('<img onerror="x">'), '&lt;img onerror=&quot;x&quot;&gt;'));
