import * as React from 'react';
import {
  Bell, CircleUserRound, Folder, House, Layers3, LayoutGrid, PanelsTopLeft,
  Plus, Search, Settings, Trash2, Star, Ellipsis, Upload, RotateCcw, X, Menu,
  type LucideIcon,
} from 'lucide-react';
import './designflow.css';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost';
export function Button({ children, variant = 'secondary', icon: Icon, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; icon?: LucideIcon }) {
  return <button type="button" className={`df-button ${props.className ?? ''}`} data-variant={variant} {...props}>{Icon && <Icon size={18} aria-hidden="true" />} {children}</button>;
}
export function IconButton({ icon: Icon, label, ...props }: Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'aria-label'> & { icon: LucideIcon; label: string }) {
  return <button type="button" className={`df-icon-button ${props.className ?? ''}`} aria-label={label} title={label} {...props}><Icon size={20} aria-hidden="true" /></button>;
}
export type NavItem = { label: string; href: string; icon: LucideIcon };
export const primaryNavigation: NavItem[] = [
  { label:'Início', href:'/', icon:House },
  { label:'Projetos', href:'/projetos', icon:Folder },
  { label:'Recursos', href:'/recursos', icon:Layers3 },
  { label:'Biblioteca', href:'/biblioteca', icon:LayoutGrid },
  { label:'Lixeira', href:'/lixeira', icon:Trash2 },
];
export const footerNavigation: NavItem[] = [
  { label:'Configurações', href:'/configuracoes', icon:Settings },
  { label:'Minha conta', href:'/conta', icon:CircleUserRound },
];
export function Sidebar({ route, onNavigate, logo, mobileOpen=false }: { route: string; onNavigate: (path:string)=>void; logo?:React.ReactNode; mobileOpen?:boolean }) {
  const item = (nav:NavItem) => <a key={nav.href} href={nav.href} className="df-nav-item" data-active={route===nav.href} aria-current={route===nav.href?'page':undefined} onClick={(e)=>{e.preventDefault(); onNavigate(nav.href);}}>
    <nav.icon size={20} strokeWidth={1.8} aria-hidden="true"/><span className="df-nav-label">{nav.label}</span>
  </a>;
  return <aside id="df-mobile-sidebar" className="df-sidebar" data-mobile-open={mobileOpen} aria-label="Navegação principal">
    <a className="df-brand" href="/" onClick={(e)=>{e.preventDefault();onNavigate('/');}}>{logo ?? <span className="df-brand-mark" aria-hidden="true">F</span>}<span className="df-brand-text">DesignFlow</span></a>
    <nav className="df-sidebar-group" aria-label="Páginas">{primaryNavigation.map(item)}</nav>
    <nav className="df-sidebar-bottom" aria-label="Conta">{footerNavigation.map(item)}</nav>
  </aside>;
}
export function SearchField({ value, onChange, placeholder='Buscar...' }: { value:string; onChange:(value:string)=>void; placeholder?:string }) {
  const ref=React.useRef<HTMLInputElement>(null);
  React.useEffect(()=>{
    const key=(e:KeyboardEvent)=>{if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='k'){e.preventDefault();ref.current?.focus();}};
    window.addEventListener('keydown',key);
    return()=>window.removeEventListener('keydown',key);
  },[]);
  return <label className="df-search"><Search size={19} aria-hidden="true"/><input ref={ref} aria-label="Buscar" placeholder={placeholder} value={value} onChange={e=>onChange(e.target.value)}/><kbd>Ctrl K</kbd></label>;
}
export function Topbar({ search, setSearch, avatarInitials='?', onNotifications, onMenuToggle }: {
  search:string; setSearch:(value:string)=>void; avatarInitials?:string; onNotifications?:()=>void; onMenuToggle?:()=>void; mobileOpen?:boolean;
}) {
  return <header className="df-topbar"><div style={{display:'flex',alignItems:'center',gap:10,minWidth:0,flex:'1 1 auto'}}>
    {onMenuToggle && <span className="df-mobile-menu-button"><IconButton icon={Menu} label="Abrir menu" aria-controls="df-mobile-sidebar" aria-expanded={Boolean(mobileOpen)} onClick={onMenuToggle}/></span>}
    <SearchField value={search} onChange={setSearch}/></div>
    <div className="df-topbar-actions"><IconButton icon={Bell} label="Notificações" onClick={onNotifications} disabled={!onNotifications}/><span className="df-avatar" aria-label="Usuário">{avatarInitials}</span></div>
  </header>;
}
export function AppShell({ route, onNavigate, children, avatarInitials='?', search, setSearch, logo }: {
  route:string; onNavigate:(path:string)=>void; children:React.ReactNode; avatarInitials?:string;
  search?:string; setSearch?:(v:string)=>void; logo?:React.ReactNode;
}) {
  const [localSearch,setLocalSearch]=React.useState('');
  const [mobileOpen,setMobileOpen]=React.useState(false);
  React.useEffect(()=>{
    if(!mobileOpen)return;
    const drawer=document.getElementById('df-mobile-sidebar');
    const trigger=document.querySelector<HTMLButtonElement>('button[aria-label="Abrir menu"]');
    const focusables=()=>Array.from(drawer?.querySelectorAll<HTMLElement>('a[href],button:not(:disabled)')??[]);
    focusables()[0]?.focus();
    const keydown=(e:KeyboardEvent)=>{
      if(e.key==='Escape'){e.preventDefault();setMobileOpen(false);}
      if(e.key==='Tab'){
        const items=focusables();if(!items.length)return;
        const first=items[0],last=items[items.length-1];
        if(e.shiftKey&&document.activeElement===first){e.preventDefault();last.focus();}
        else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first.focus();}
      }
    };
    document.addEventListener('keydown',keydown);
    return()=>{document.removeEventListener('keydown',keydown);trigger?.focus();};
  },[mobileOpen]);
  const effectiveSearch=search ?? localSearch;
  const effectiveSetSearch=setSearch ?? setLocalSearch;
  const navigate=(path:string)=>{setMobileOpen(false);onNavigate(path)};
  return <div className="df-shell">
    <a href="#main-content" className="df-skip-link">Pular para conteúdo</a>
    {mobileOpen && <button type="button" className="df-sidebar-backdrop" aria-label="Fechar menu" onClick={()=>setMobileOpen(false)}/>}
    <Sidebar route={route} onNavigate={navigate} mobileOpen={mobileOpen} logo={logo}/>
    <div className="df-main"><Topbar search={effectiveSearch} setSearch={effectiveSetSearch} avatarInitials={avatarInitials} onMenuToggle={()=>setMobileOpen(true)} mobileOpen={mobileOpen}/>
      <main className="df-page" id="main-content" tabIndex={-1}>{children}</main></div>
  </div>;
}
export function PageHeader({ title, description, actions }: { title:string; description?:string; actions?:React.ReactNode }) {
  return <div className="df-page-header"><div><h1 className="df-page-title">{title}</h1>{description && <p className="df-page-subtitle">{description}</p>}</div>{actions}</div>;
}
export function SegmentedTabs<T extends string>({ items, value, onChange }: { items:readonly { label:string; value:T; icon?:LucideIcon }[]; value:T; onChange:(value:T)=>void }) {
  return <div className="df-tabs" role="group" aria-label="Filtros">{items.map(tab=><button type="button" key={tab.value} className="df-tab" aria-pressed={value===tab.value} data-active={value===tab.value} onClick={()=>onChange(tab.value)}>{tab.icon && <tab.icon size={18} aria-hidden="true"/>}{tab.label}</button>)}</div>;
}
export function ContentCard({ children, className='', ...props }:React.HTMLAttributes<HTMLElement>) {
  return <article {...props} className={`df-card ${className}`}>{children}</article>;
}
export function TemplateCard({ title, description, thumbnail, onOpen }: {title:string;description?:string;thumbnail?:React.ReactNode;onOpen:()=>void}) {
  return <ContentCard><div className="df-card-preview">{thumbnail ?? <PanelsTopLeft size={50} strokeWidth={1.2}/>}</div><div className="df-card-body df-card-actions"><div><h3 className="df-card-title">{title}</h3>{description && <p className="df-card-subtitle">{description}</p>}</div><Button icon={Plus} onClick={onOpen}>Usar</Button></div></ContentCard>;
}
export function ProjectCard({ title, lastEdited, thumbnail, favorite=false, onOpen, onFavorite, onMore }: { title:string;lastEdited:string;thumbnail?:React.ReactNode;favorite?:boolean;onOpen:()=>void;onFavorite:()=>void;onMore:()=>void }) {
  return <ContentCard><button type="button" className="df-card-preview df-card-preview-button" aria-label={`Abrir ${title}`} onClick={onOpen}>{thumbnail ?? <Folder size={52} strokeWidth={1.4}/>}</button><div className="df-card-body df-card-actions"><div><h3 className="df-card-title">{title}</h3><p className="df-card-subtitle">{lastEdited}</p></div><div style={{display:'flex'}}><IconButton icon={Star} label={favorite?'Remover favorito':'Adicionar favorito'} aria-pressed={favorite} onClick={onFavorite}/><IconButton icon={Ellipsis} label={`Ações para ${title}`} onClick={onMore}/></div></div></ContentCard>;
}
export function ResourceCard({ title, description, preview, onOpen }: { title:string;description:string;preview?:React.ReactNode;onOpen:()=>void }) {
  return <ContentCard><div className="df-card-preview">{preview ?? <Layers3 size={52} strokeWidth={1.4}/>}</div><div className="df-card-body df-card-actions"><div><h3 className="df-card-title">{title}</h3><p className="df-card-subtitle">{description}</p></div><Button onClick={onOpen}>Explorar</Button></div></ContentCard>;
}

export function LibraryCard({ title, itemCount, thumbnail, isFavorite=false, onOpen, onFavorite, onMore }: { title:string;itemCount:number;thumbnail?:React.ReactNode;isFavorite?:boolean;onOpen:()=>void;onFavorite:()=>void;onMore:()=>void }) {
  return <ContentCard><button type="button" className="df-card-preview df-card-preview-button" aria-label={`Abrir coleção ${title}`} onClick={onOpen}>{thumbnail ?? <LayoutGrid size={52} strokeWidth={1.4}/>}</button><div className="df-card-body df-card-actions"><div><h3 className="df-card-title">{title}</h3><p className="df-card-subtitle">{itemCount} {itemCount===1?'item':'itens'}</p></div><div style={{display:'flex'}}><IconButton icon={Star} label={isFavorite?'Remover favorito':'Adicionar favorito'} aria-pressed={isFavorite} onClick={onFavorite}/><IconButton icon={Ellipsis} label={`Mais opções para ${title}`} onClick={onMore}/></div></div></ContentCard>;
}
export function AccountSummary({ displayName, email, avatarUrl, onEdit }: { displayName:string;email:string;avatarUrl?:string;onEdit:()=>void }) {
  const initials=displayName.trim().split(/\s+/).slice(0,2).map(w=>w.charAt(0).toUpperCase()).join('') || '?';
  return <ContentCard><div className="df-card-body df-account-summary"><div className="df-account-avatar">{avatarUrl?<img src={avatarUrl} alt="Foto de perfil"/>:initials}</div><div><h2 className="df-card-title">{displayName}</h2><p className="df-card-subtitle" style={{marginBottom:18}}>{email}</p><Button onClick={onEdit}>Editar perfil</Button></div></div></ContentCard>;
}
export function Toast({ message, tone='success', onDismiss }: { message:string;tone?:'success'|'error'|'info';onDismiss?:()=>void }) {
  return <div className="df-toast" data-tone={tone} role={tone==='error'?'alert':'status'}><span>{message}</span>{onDismiss && <IconButton icon={X} label="Fechar aviso" onClick={onDismiss}/>}</div>;
}
export function Modal({ title, description, onClose, children }: { title:string;description?:string;onClose:()=>void;children:React.ReactNode }) {
  const closeRef=React.useRef<HTMLButtonElement>(null);
  React.useEffect(()=>{const old=document.activeElement as HTMLElement|null;closeRef.current?.focus(); const escape=(e:KeyboardEvent)=>{if(e.key==='Escape')onClose();};document.addEventListener('keydown',escape);return()=>{document.removeEventListener('keydown',escape);old?.focus();};},[onClose]);
  return <div className="df-modal-backdrop" onMouseDown={e=>{if(e.target===e.currentTarget)onClose();}}><section className="df-modal" role="dialog" aria-modal="true" aria-label={title}><div className="df-modal-head"><div><h2>{title}</h2>{description && <p>{description}</p>}</div><button type="button" ref={closeRef} className="df-modal-close" aria-label="Fechar" onClick={onClose}>×</button></div>{children}</section></div>;
}

export function FileRow({ name, description, icon: Icon=Folder, selected, onSelect, onRestore, onMore }: { name:string;description?:string;icon?:LucideIcon;selected?:boolean;onSelect?:(checked:boolean)=>void;onRestore?:()=>void;onMore?:()=>void }) {
  return <div className="df-file-row">{onSelect && <input type="checkbox" checked={!!selected} onChange={e=>onSelect(e.target.checked)} aria-label={`Selecionar ${name}`}/>}<Icon size={24} color="var(--df-primary)" aria-hidden="true"/><div className="df-file-row-main"><p className="df-file-row-name">{name}</p>{description && <p className="df-file-row-description">{description}</p>}</div>{onRestore && <IconButton icon={RotateCcw} label={`Restaurar ${name}`} onClick={onRestore}/>} {onMore && <IconButton icon={Ellipsis} label={`Mais opções para ${name}`} onClick={onMore}/>}</div>;
}
export function SettingsRow({ title, description, children }: {title:string;description?:string;children:React.ReactNode}) {
  return <div className="df-setting-row"><div><h3>{title}</h3>{description && <p>{description}</p>}</div>{children}</div>;
}
export function Switch({ checked, onChange, label }: {checked:boolean;onChange:(checked:boolean)=>void;label:string}) {
  return <input type="checkbox" role="switch" aria-label={label} className="df-switch" checked={checked} onChange={e=>onChange(e.target.checked)}/>;
}
export function EmptyState({ title, description, onPrimary, secondary }: {title:string;description:string;onPrimary:()=>void;secondary?:React.ReactNode}) {
  return <div className="df-empty"><div className="df-empty-graphic" aria-hidden="true"><div className="df-empty-sheet"/><div className="df-empty-sheet"/><div className="df-empty-sheet"><Plus size={32}/></div></div><h2>{title}</h2><p>{description}</p><Button variant="primary" icon={Plus} onClick={onPrimary}>Novo projeto</Button>{secondary && <div className="df-empty-options">{secondary}</div>}</div>;
}
export function UploadDropzone({ onFiles, accept='.png,.jpg,.jpeg,.svg,.pdf', formats='PNG, JPG, SVG e PDF' }: {onFiles:(files:File[])=>void;accept?:string;formats?:string}) {
  const input=React.useRef<HTMLInputElement>(null);
  const [drag,setDrag]=React.useState(false);
  return <section aria-label="Importar arquivos" onDragOver={e=>{e.preventDefault();setDrag(true)}} onDragLeave={()=>setDrag(false)} onDrop={e=>{e.preventDefault();setDrag(false);onFiles(Array.from(e.dataTransfer.files));}} style={{padding:'70px 32px',border:`1px dashed ${drag?'var(--df-primary)':'#CAB3FF'}`,borderRadius:16,textAlign:'center',background:'rgba(255,255,255,.35)'}}>
    <Upload color="var(--df-primary)" size={42} strokeWidth={1.4}/><h2>Arraste arquivos aqui</h2><p className="df-page-subtitle" style={{marginBottom:24}}>Ou selecione um arquivo do seu computador.</p><input ref={input} type="file" multiple hidden accept={accept} onChange={e=>{onFiles(Array.from(e.target.files??[]));e.target.value='';}}/><Button variant="primary" icon={Upload} onClick={()=>input.current?.click()}>Selecionar arquivo</Button><p className="df-card-subtitle" style={{marginTop:18}}>Formatos aceitos: {formats}</p>
  </section>;
}
