const toggle=document.querySelector("#enabled");
const status=document.querySelector("#status");
const id=document.querySelector("#extension-id");
id.textContent=chrome.runtime.id;
chrome.storage.local.get({enabled:false}).then(({enabled})=>{toggle.checked=enabled;update(enabled);});
function update(enabled){status.textContent=enabled?"Detección habilitada. Meeting Recorder también debe tener su interruptor activado.":"Detección desactivada. No se iniciará ninguna grabación.";}
toggle.addEventListener("change", async()=>{await chrome.storage.local.set({enabled:toggle.checked});update(toggle.checked);});
document.querySelector("#copy").addEventListener("click",()=>navigator.clipboard.writeText(chrome.runtime.id));
