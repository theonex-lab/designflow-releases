/** Domínio demonstrativo; adapte aos tipos do backend existente. Nenhum endpoint é presumido. */
export type ID = string;
export type ISODate = string;
export type AppRoute = '/' | '/modelos' | '/importar' | '/projetos' | '/recursos' | '/biblioteca' | '/lixeira' | '/conta' | '/configuracoes' | `/editor/${string}`;
export type ProjectStatus = 'draft' | 'active' | 'archived';
export type DataState<T> =
  | { status:'idle' }
  | { status:'loading' }
  | { status:'ready'; data:T }
  | { status:'empty'; data:T }
  | { status:'error'; error:{message:string; code?:string; retryable:boolean} };
export interface Project { id:ID; title:string; status:ProjectStatus; updatedAt:ISODate; createdAt:ISODate; favorite:boolean; thumbnailUrl?:string; }
export type TemplateCategory = 'social'|'presentation'|'web'|'resume'|'moodboard'|'brand';
export interface Template { id:ID; title:string; category:TemplateCategory; description?:string; thumbnailUrl?:string; supported:boolean; }
export type AssetCategory = 'icon'|'illustration'|'mockup'|'component'|'image'|'font'|'other';
export interface CatalogResource { id:ID; title:string; category:AssetCategory; previewUrl?:string; licenseUrl?:string; }
export interface UserLibraryItem { id:ID; title:string; type:'upload'|'collection'|'logo'|'palette'|'image'|'other'; ownerId:ID; itemCount?:number; favorite?:boolean; previewUrl?:string; }
export interface TrashItem { id:ID; title:string; kind:'project'|'file'; deletedAt:ISODate; previewUrl?:string; }
export interface Account { id:ID; name:string; email:string; avatarUrl?:string; subscription?:{name:string; manageUrl?:string}; storage?:{usedBytes:number; limitBytes:number}; }
export type ThemePreference = 'light'|'dark'|'system';
export interface Preferences { workspaceName:string; autosave:boolean; defaultRoute:Exclude<AppRoute, `/editor/${string}`>; theme:ThemePreference; locale:string; emailNotifications:boolean; }
export interface UploadCapability { allowedExtensions:readonly string[]; maxFileBytes:number; allowMultiple:boolean; }
export interface Capabilities {
  upload?:UploadCapability;
  templateCreation:boolean;
  canTrashProjects:boolean;
  canPermanentlyDelete:boolean;
  canChangePassword:boolean;
  canManage2FA:boolean;
  canListActiveSessions:boolean;
  canExport:boolean;
  subscriptionVisible:boolean;
  storageQuotaVisible:boolean;
}
export interface AppAdapters {
  listProjects():Promise<Project[]>;
  createBlankProject(name?:string):Promise<Project>;
  createFromTemplate?(templateId:ID):Promise<Project>;
  updateFavorite?(projectId:ID, favorite:boolean):Promise<Project>;
  trashProject?(id:ID):Promise<void>;
  listTemplates?():Promise<Template[]>;
  listCatalog?():Promise<CatalogResource[]>;
  listLibrary?():Promise<UserLibraryItem[]>;
  listTrash?():Promise<TrashItem[]>;
  restoreTrash?(ids:readonly ID[]):Promise<void>;
  emptyTrash?():Promise<void>;
  uploadFiles?(files:File[]):Promise<UserLibraryItem[]>;
  loadAccount?():Promise<Account>;
  loadPreferences?():Promise<Preferences>;
  savePreferences?(preferences:Preferences):Promise<Preferences>;
}
