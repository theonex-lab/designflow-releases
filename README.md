# DesignFlow

Editor nativo de design e prototipagem em Rust/egui, com espaço pessoal local, identidade branca/roxa e documentos editáveis.

## Abrir e desenvolver

Abra o atalho `DesignFlow.lnk` na raiz, depois de compilar:

```text
cargo build -p designflow --bin designflow --offline
cargo run -p designflow --bin designflow --offline
```

O executável fica em `target/debug/designflow.exe`. As instruções de alteração do código estão em [INSTRUCOES_DESIGNFLOW.txt](INSTRUCOES_DESIGNFLOW.txt).

## Especificação visual e contratos

Leia [assets.md](assets.md) e os documentos numerados em [docs](docs). A ordem de leitura e as prioridades estão no contrato principal. Imagens em `assets/references` são referências de composição, nunca telas rasterizadas usadas como interface.

- [Design system](docs/01-design-system.md)
- [Componentes](docs/02-components.md)
- [Telas](docs/03-screens.md)
- [Fluxos](docs/04-flows.md)
- [Responsividade e acessibilidade](docs/05-responsive-a11y.md)
- [Dados e capacidades](docs/06-data-contracts.md)
- [Aceitação](docs/07-acceptance.md)

`assets/design-tokens.json` é a fonte de medidas e cores: o build nativo gera constantes Rust; `scripts/generate_css_tokens.py` mantém o CSS de referência sincronizado. Os componentes React/TypeScript fornecidos são referências portadas ao framework nativo, não um segundo aplicativo.

## Operação real e limites

O espaço pessoal usa arquivos locais por conta. Criação em branco, modelos nativos, importação validada, favoritos, biblioteca, lixeira, perfil e preferências usam um catálogo persistente e cópias gerenciadas. Os originais importados são preservados. A conta é local e não sincroniza entre computadores.

Não há backend de assinatura, cobrança, quota, notificações por e-mail, 2FA ou catálogo remoto. Essas áreas não recebem dados fictícios. Os modelos incluídos contêm exemplos editáveis explicitamente identificados. `examples/demo-data.ts` é somente demonstração isolada e não alimenta o app.

O adaptador de importação do hub aceita `.designflow`, `.svg`, `.png`, `.jpg`, `.jpeg` e `.webp`, com validação pelo motor e limite local efetivo de 256 MiB. Não anuncia importação de PDF ou PSD. O editor mantém seus recursos nativos de exportação, camadas, texto, componentes, auto layout e prototipagem.

Os documentos técnicos não numerados registram também a história da base do editor; menções antigas a CLI, servidor e distribuição web não significam que esses aplicativos estejam incluídos nesta pasta.

## Verificar o pacote

```text
python scripts/generate_css_tokens.py --check
python checks/validate_package.py
cargo test -p designflow-ui-egui --lib workspace::tests --offline
```

## Licenças

A base de código mantém [MIT](LICENSE-MIT) ou [Apache-2.0](LICENSE-APACHE). Avisos e atribuições permanecem em [NOTICE](NOTICE) e [ATTRIBUTION.md](ATTRIBUTION.md). A marca DesignFlow e as referências foram fornecidas pelo proprietário desta cópia.
