# UI React / TypeScript

Referência: `DesignFlowUI.tsx` + `AdvancedUI.tsx`, estilos `designflow.css` + `advanced.css`, tokens gerados em `tokens.generated.css`. Dependências de runtime: `react` e `lucide-react`; o projeto integrador deve fornecer React/TSX e build CSS.

O pacote contém componentes de referência, não toda a aplicação. A tela `HomeExample.tsx` ilustra composição do hub; `../../examples/CompositionExample.tsx` ilustra fluxos **somente locais**, sem backend. Prefira `ValidatedDropzone` a `UploadDropzone` (legado, não valida tipo/tamanho) e `ConfirmDialog` a `Modal` para operações destrutivas.

Para construção de verdade, usar roteador e dados já presentes no projeto. No mobile, a sidebar usa drawer, não barra de navegação horizontal.
