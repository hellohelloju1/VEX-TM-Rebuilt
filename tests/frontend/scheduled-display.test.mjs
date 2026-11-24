import {test} from 'node:test';import assert from 'node:assert/strict';import {Window} from 'happy-dom';import {startDisplay} from '../../frontend/displays.js';
test('audience countdown refreshes without rebuilding the scrolling rankings',async()=>{
 const win=new Window();win.document.body.innerHTML='<div id="app"></div>';
 const previous={window:globalThis.window,document:globalThis.document,requestAnimationFrame:globalThis.requestAnimationFrame,setInterval:globalThis.setInterval};let refresh;
 Object.assign(globalThis,{window:win,document:win.document,requestAnimationFrame:()=>0,setInterval:fn=>{refresh=fn;return 0;}});
 const s={scheduled_start_ms:120000,server_time_ms:0,active:1,timer:{phase:'Idle',seconds:120},rankings:[],field_control:{audience:'Rankings'},event:{name:'Test',game:'VRC_2026_Override',teams:[],skills:[],matches:[{id:1,state:'Unplayed',field:'Field 1'}],config:{displays:[{id:1,kind:'Audience'}],sponsors:[]}}};
 try {await startDisplay(1,async()=>s);assert.equal(document.querySelector('[data-scheduled-start]').textContent,'Scheduled start in: 02:00');const table=document.querySelector('#pit-unit');assert.ok(table);s.server_time_ms=1000;await refresh();assert.equal(document.querySelector('[data-scheduled-start]').textContent,'Scheduled start in: 01:59');assert.equal(document.querySelector('#pit-unit'),table);s.timer.phase='Countdown';await refresh();assert.equal(document.querySelector('[data-scheduled-start]').hidden,true);}finally{Object.assign(globalThis,previous);await win.happyDOM.close();}
});
