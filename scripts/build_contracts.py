#!/usr/bin/env python3
"""Create agent-friendly machine-readable screen and flow contracts."""
import json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
base={
 'schemaVersion':'2.0','product':'DesignFlow','language':'pt-BR','visualRules':{
  'lightThemeDefault':True,'logoPosition':'top-left-only','editorLogoOnCanvas':False,
  'minimalist':True,'avoid':['large central brand','decorative overload','excessive shadows','fake data','dead buttons','zoomed-in UI'],
  'sidebarNavigation':['/','/projetos','/recursos','/biblioteca','/lixeira','/configuracoes','/conta']},
 'screens':[
  {'key':'home','route':'/','reference':'assets/references/00-inicio.png','title':'Bem-vindo ao DesignFlow','shell':'hub','primaryAction':'create-blank','sections':['page-heading','empty-state-or-continue'], 'emptyCondition':'projects.length === 0','filters':[], 'requiredStates':['loading','empty','ready','error']},
  {'key':'templates','route':'/modelos','reference':'assets/references/01-modelos.png','title':'Escolha um modelo','shell':'hub','primaryAction':'preview-template','sections':['page-heading','category-filters','template-grid'],'filters':['all','social','presentation','web','resume','moodboard','brand'],'requiredStates':['loading','empty','ready','error','filtered-empty']},
  {'key':'import','route':'/importar','reference':'assets/references/02-importar.png','title':'Importar','shell':'hub','primaryAction':'select-file','sections':['page-heading','dropzone','recent-imports-optional'],'filters':[], 'requiredStates':['idle','validating','uploading','success','error']},
  {'key':'projects','route':'/projetos','reference':'assets/references/03-projetos.png','title':'Seus projetos','shell':'hub','primaryAction':'create-blank','sections':['page-heading','filters','grid-or-list'],'filters':['recent','in-progress','favorites'],'requiredStates':['loading','empty','ready','error','filtered-empty','deleting']},
  {'key':'resources','route':'/recursos','reference':'assets/references/04-recursos.png','title':'Recursos','shell':'hub','primaryAction':'explore-resource','sections':['page-heading','category-filters','resource-grid'],'filters':['all','icons','illustrations','mockups','components'],'requiredStates':['loading','empty','ready','error','filtered-empty']},
  {'key':'library','route':'/biblioteca','reference':'assets/references/05-biblioteca.png','title':'Biblioteca','shell':'hub','primaryAction':'add-to-library-if-supported','sections':['page-heading','category-filters','collection-grid'],'filters':['all','favorites','uploads','logos','colors','images'],'requiredStates':['loading','empty','ready','error','filtered-empty']},
  {'key':'trash','route':'/lixeira','reference':'assets/references/06-lixeira.png','title':'Lixeira','shell':'hub','primaryAction':'restore-selection','sections':['page-heading','filters','file-list'],'filters':['all','projects','files'],'requiredStates':['loading','empty','ready','error','confirming','deleting','restoring']},
  {'key':'account','route':'/conta','reference':'assets/references/07-minha-conta.png','title':'Minha conta','shell':'hub','primaryAction':'edit-profile','sections':['page-heading','profile','subscription-if-available','storage-if-available','security-if-available'],'filters':[],'requiredStates':['loading','ready','error','saving']},
  {'key':'settings','route':'/configuracoes','reference':'assets/references/08-configuracoes.png','title':'Configurações','shell':'hub','primaryAction':'save-changes','sections':['page-heading','local-navigation','settings-form'],'filters':['general','appearance','notifications','shortcuts','language','privacy'],'requiredStates':['loading','ready','dirty','saving','saved','error']},
  {'key':'editor','route':'/editor/:projectId','reference':'assets/references/09-editor.png','title':'Editor','shell':'editor','primaryAction':'edit-selected-layer','sections':['editor-topbar','layers-panel','canvas','properties-panel','bottom-toolbar'],'filters':[],'requiredStates':['loading','ready','empty-document','saving','error','readonly']},
 ]}
flows={
 'schemaVersion':'2.0','events':[
  {'id':'create-blank','from':['/','/projetos'],'trigger':'Novo projeto/Em branco','preconditions':['user-authenticated'],'pending':'creating','success':'/editor/{createdProject.id}','error':'Preserve previous screen and show retry'},
  {'id':'preview-template','from':['/modelos'],'trigger':'Click template','pending':'opening-preview','success':'preview-dialog','error':'Show template unavailable'},
  {'id':'create-template','from':['preview-dialog'],'trigger':'Usar modelo','pending':'creating','success':'/editor/{createdProject.id}','error':'Keep preview and show retry'},
  {'id':'select-file','from':['/importar'],'trigger':'drop-or-file-picker','preconditions':['file matches server capabilities'],'pending':'uploading','success':'update-recent-imports','error':'Keep file context; show validation/server error'},
  {'id':'toggle-favorite','from':['/projetos','/biblioteca'],'trigger':'Favorite button','pending':'saving','success':'update-favorite','error':'rollback and toast'},
  {'id':'trash-project','from':['/projetos'],'trigger':'Delete from contextual menu','pending':'confirming-then-deleting','success':'move-to-trash','error':'keep card and show error'},
  {'id':'restore-selection','from':['/lixeira'],'trigger':'Restaurar selecionados','preconditions':['selectedIds.length > 0'],'pending':'restoring','success':'remove restored from trash','error':'show partial failures'},
  {'id':'empty-trash','from':['/lixeira'],'trigger':'Esvaziar lixeira','pending':'confirming-then-deleting','success':'empty list','error':'keep list and show error'},
  {'id':'save-settings','from':['/configuracoes'],'trigger':'Salvar alterações','preconditions':['dirty and valid'],'pending':'saving','success':'saved-indicator','error':'keep dirty changes + retry'},
  {'id':'edit-layer','from':['/editor/:projectId'],'trigger':'Change selected properties','preconditions':['project loaded and can edit'],'pending':'local-change-then-saving','success':'saved-status','error':'unsaved-status + retry'},
 ]}
(root/'assets/screen-contracts.json').write_text(json.dumps(base,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(root/'assets/flows.json').write_text(json.dumps(flows,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Generated machine-readable screen and flow contracts')
