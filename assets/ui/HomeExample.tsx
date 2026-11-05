import * as React from 'react';
import { FileText, LayoutGrid, Upload } from 'lucide-react';
import { AppShell, PageHeader, Button, EmptyState } from './DesignFlowUI';

/** Exemplo de composição. Troque os callbacks pelos fluxos de criação/importação da sua aplicação. */
export function HomeExample({ onCreate, onTemplates, onImport }: { onCreate:()=>void; onTemplates:()=>void; onImport:()=>void }) {
  const [route,setRoute]=React.useState('/');
  const [search,setSearch]=React.useState('');
  return <AppShell route={route} onNavigate={setRoute} search={search} setSearch={setSearch}>
    <PageHeader title="Bem-vindo ao DesignFlow" description="Crie, organize e desenvolva seus projetos em um fluxo simples e profissional." />
    <EmptyState title="Crie seu primeiro projeto" description="Comece do zero, use um modelo ou importe um arquivo." onPrimary={onCreate} secondary={<>
      <Button icon={FileText} onClick={onCreate}>Em branco</Button>
      <Button icon={LayoutGrid} onClick={onTemplates}>Modelo</Button>
      <Button icon={Upload} onClick={onImport}>Importar</Button>
    </>}/>
  </AppShell>;
}
