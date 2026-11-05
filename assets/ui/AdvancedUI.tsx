import * as React from 'react';
import { AlertCircle, CircleCheck, LoaderCircle, Upload, X } from 'lucide-react';
import { Button, IconButton } from './DesignFlowUI';
import './advanced.css';

/** Categoria de filtro: não use role=tab quando não há tabpanel/controle de foco de tabs. */
export function FilterChips<T extends string>({ items, value, onChange, label = 'Filtros' }: {
  items: readonly { label:string; value:T }[];
  value:T; onChange:(value:T)=>void; label?:string;
}) {
  return <div role="group" aria-label={label} className="df-filter-chips">{items.map(item =>
    <button key={item.value} type="button" className="df-filter-chip"
      aria-pressed={item.value === value} onClick={()=>onChange(item.value)}>{item.label}</button>)}</div>;
}

export function StatusView({ state, message, onRetry, children }: {
  state:'loading'|'empty'|'error'|'ready'; message?:string; onRetry?:()=>void; children?:React.ReactNode;
}) {
  if(state === 'ready') return <>{children}</>;
  return <div className="df-status-view" role={state==='error'?'alert':'status'} aria-busy={state==='loading'}>
    {state==='loading' && <LoaderCircle aria-hidden="true" className="df-spinner" size={25}/>}
    {state==='error' && <AlertCircle aria-hidden="true" size={25}/>}
    {state==='empty' && <CircleCheck aria-hidden="true" size={25}/>}
    <p>{message ?? ({loading:'Carregando...',empty:'Nenhum item encontrado.',error:'Não foi possível carregar.',ready:''}[state])}</p>
    {state==='error' && onRetry && <Button onClick={onRetry}>Tentar novamente</Button>}
  </div>;
}

export function TextField({ id, label, hint, error, ...props }: React.InputHTMLAttributes<HTMLInputElement> & {
  id:string; label:string; hint?:string; error?:string;
}) {
  return <div className="df-form-field"><label htmlFor={id}>{label}</label>
    <input id={id} className="df-form-control" aria-invalid={Boolean(error)}
      aria-describedby={error ? `${id}-error` : hint ? `${id}-hint` : undefined} {...props}/>
    {error ? <p className="df-field-error" id={`${id}-error`}>{error}</p> : hint ? <p className="df-field-hint" id={`${id}-hint`}>{hint}</p> : null}
  </div>;
}

/** Diálogo acessível: prende foco, Escape fecha, restaura o foco do gatilho. */
export function ConfirmDialog({ open, title, description, confirmLabel='Confirmar', danger=false,
  pending=false, error, onConfirm, onClose }: {
  open:boolean; title:string; description:string; confirmLabel?:string; danger?:boolean;
  pending?:boolean; error?:string; onConfirm:()=>void; onClose:()=>void;
}) {
  const dialogRef=React.useRef<HTMLDivElement>(null);
  const cancelRef=React.useRef<HTMLButtonElement>(null);
  React.useEffect(()=>{
    if(!open)return;
    const before=document.activeElement as HTMLElement|null;
    cancelRef.current?.focus();
    const handle=(e:KeyboardEvent)=>{
      if(e.key==='Escape'){e.preventDefault();if(!pending)onClose();}
      if(e.key==='Tab'){
        const focusables=Array.from(dialogRef.current?.querySelectorAll<HTMLElement>('button:not(:disabled),[href],input:not(:disabled),select:not(:disabled),[tabindex]:not([tabindex="-1"])')??[]);
        if(!focusables.length){e.preventDefault();return;}
        const first=focusables[0],last=focusables[focusables.length-1];
        if(e.shiftKey && document.activeElement===first){e.preventDefault();last.focus();}
        if(!e.shiftKey && document.activeElement===last){e.preventDefault();first.focus();}
      }
    };
    document.addEventListener('keydown',handle);
    return()=>{document.removeEventListener('keydown',handle);before?.focus();};
  },[open,pending,onClose]);
  if(!open) return null;
  return <div className="df-confirm-overlay" onMouseDown={e=>{if(e.currentTarget===e.target && !pending)onClose();}}>
    <div ref={dialogRef} className="df-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="df-confirm-title" aria-describedby="df-confirm-description" aria-busy={pending}>
      <div className="df-confirm-title-row"><h2 id="df-confirm-title">{title}</h2><IconButton icon={X} label="Fechar diálogo" onClick={onClose} disabled={pending}/></div>
      <p id="df-confirm-description">{description}</p>
      {error && <p role="alert" className="df-field-error">{error}</p>}
      <div className="df-confirm-actions"><button className="df-button" data-variant="secondary" ref={cancelRef} disabled={pending} onClick={onClose}>Cancelar</button>
      <button className="df-button" data-variant={danger?'danger':'primary'} disabled={pending} onClick={onConfirm}>{pending?'Aguarde...':confirmLabel}</button></div>
    </div>
  </div>;
}

const extensions = (accept:string) => accept.toLowerCase().split(',').map(x=>x.trim()).filter(x=>x.startsWith('.'));
/** Apenas validação de UX; o servidor precisa validar formato, conteúdo e tamanho. */
export function ValidatedDropzone({ allowedExtensions, maxFileBytes, multiple=false, onFiles, onError, pending=false }: {
  allowedExtensions:readonly string[]; maxFileBytes:number; multiple?:boolean;
  onFiles:(files:File[])=>void; onError:(message:string)=>void; pending?:boolean;
}) {
  const ref=React.useRef<HTMLInputElement>(null);
  const [dragging,setDragging]=React.useState(false);
  const accept=allowedExtensions.join(',');
  const validate=(files:File[])=>{
    if(!files.length)return;
    if(!multiple && files.length>1){onError('Selecione apenas um arquivo.');return;}
    const expected=extensions(accept);
    const bad=files.find(f=>!expected.some(ext=>f.name.toLowerCase().endsWith(ext)));
    if(bad){onError(`Formato não permitido: ${bad.name}.`);return;}
    const tooBig=files.find(f=>f.size>maxFileBytes);
    if(tooBig){onError(`Arquivo excede o limite de ${(maxFileBytes/1024/1024).toFixed(0)} MB: ${tooBig.name}.`);return;}
    onFiles(files);
  };
  return <section className="df-dropzone" data-dragging={dragging} aria-label="Importação de arquivos" aria-busy={pending}
    onDragEnter={e=>{e.preventDefault();setDragging(true);}}
    onDragOver={e=>e.preventDefault()} onDragLeave={e=>{if(e.currentTarget===e.target)setDragging(false);}}
    onDrop={e=>{e.preventDefault();setDragging(false);if(!pending)validate(Array.from(e.dataTransfer.files));}}>
    <Upload size={36} aria-hidden="true"/>
    <h2>Arraste arquivos aqui</h2>
    <p>Ou selecione do seu computador.</p>
    <input hidden ref={ref} type="file" accept={accept} multiple={multiple} disabled={pending}
      onChange={e=>{validate(Array.from(e.currentTarget.files??[]));e.currentTarget.value='';}}/>
    <Button variant="primary" onClick={()=>ref.current?.click()} disabled={pending}>{pending?'Importando...':'Selecionar arquivo'}</Button>
    <small>Formatos: {allowedExtensions.join(', ').toUpperCase()} · Limite: {(maxFileBytes/1024/1024).toFixed(0)} MB por arquivo</small>
  </section>;
}
