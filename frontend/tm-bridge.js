window.__TM_REMOTE__=true;
if(typeof sessionStorage!=='undefined'&&typeof location!=='undefined'&&new URLSearchParams(location.search).get('client')==='1')sessionStorage.setItem('tm-desktop-client','1');
if(typeof document!=='undefined')document.addEventListener('DOMContentLoaded',()=>{if(new URLSearchParams(location.search).has('control')&&window.matchMedia?.('(max-width: 760px)').matches)return;const link=document.createElement('button');link.textContent='Sign out';link.className='tm-signout';link.onclick=async()=>{await fetch('/TM/logout',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}',credentials:'same-origin'});location.reload();};document.body.append(link);});
window.__TAURI__={core:{invoke:async(_,{action,payload})=>{
 if(action==='field_control'){if(typeof sessionStorage!=='undefined'&&sessionStorage.getItem('tm-desktop-client')==='1'){location.href='/TM/?control=1&client=1';return;}if(window.matchMedia?.('(max-width: 760px)').matches){location.href='/TM/?control=1';return;}window.open('/TM/?control=1','tm-field-control','width=1000,height=720');return;}
 const response=await fetch('/TM/api',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({action,payload:payload||{}}),credentials:'same-origin'});
 if(!response.ok){if(response.status===401){if(typeof location!=='undefined')location.reload();throw Error('Tournament password required. Reload this page to sign in.');}let message;try{message=(await response.json()).error;}catch{}throw Error(message||'Server request failed');}return response.json();
}}};
