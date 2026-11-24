import {startDisplay} from './displays.js';
import {escapeHtml as h} from './planner.js';
export function displayPicker(s){
 return `<main class="display-picker"><h1>${h(s.event.name)}</h1><h2>Choose a display</h2><div class="display-picker-grid">${s.event.config.displays.map(d=>`<button class="display-choice" data-remote-display="${d.id}"><strong>${h(d.name)}</strong><span>${h(d.kind)}${d.kind==='Field'&&d.field?` · ${h(d.field)}`:''}</span></button>`).join('')||'<p>No displays configured.</p>'}</div><p id="remote-error" role="alert" hidden></p></main>`;
}
export async function enterFullscreen(doc=document){
 const target=doc.documentElement;
 if(doc.fullscreenElement||doc.webkitFullscreenElement)return;
 const request=target.requestFullscreen||target.webkitRequestFullscreen;
 if(request)await request.call(target);
}
async function fetchState(){const r=await fetch('/state',{cache:'no-store',credentials:'same-origin'});if(r.status===401){location.reload();throw Error('Sign in required');}if(!r.ok)throw Error('Display server unavailable');return r.json();}
export async function initRemote(){
 const root=document.querySelector('#app'),s=await fetchState();
 async function show(id,fullscreen=false){
  if(!s.event.config.displays.some(d=>d.id===id))throw Error('Display not found');
  const request=fullscreen?enterFullscreen().catch(()=>{}):Promise.resolve();
  history.replaceState(null,'',`/?display=${id}`);
  await startDisplay(id,fetchState);await request;
  const toolbar=document.createElement('nav');toolbar.className='remote-display-toolbar';toolbar.innerHTML='<a href="/">Displays</a><button id="remote-fullscreen">Fullscreen</button>';
  document.body.append(toolbar);toolbar.querySelector('button').addEventListener('click',()=>enterFullscreen().catch(()=>{}));
 }
 const requested=new URLSearchParams(location.search).get('display');
 if(requested){await show(+requested);return;}
 root.innerHTML=displayPicker(s);
 root.addEventListener('click',e=>{const button=e.target.closest('[data-remote-display]');if(!button)return;button.disabled=true;show(+button.dataset.remoteDisplay,true).catch(error=>{const target=document.querySelector('#remote-error');if(target){target.hidden=false;target.textContent=String(error);}button.disabled=false;});});
}
if(typeof document!=='undefined')initRemote().catch(e=>{document.querySelector('#app').textContent=String(e);});
