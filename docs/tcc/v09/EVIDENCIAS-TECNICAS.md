# Evidencias tecnicas da v09

Verificacoes executadas em 9 de agosto de 2026 no estado de trabalho usado para produzir o TCC v09.

## Interface

- `yarn lint`: aprovado sem avisos;
- `yarn typecheck`: aprovado;
- `yarn test`: 17 testes aprovados em 9 arquivos;
- `yarn build`: aprovado;
- alerta nao impeditivo: dois chunks de producao excedem 500 kB.

## Nucleo Rust

- `cargo fmt --all -- --check`: aprovado;
- `cargo clippy --all-targets --all-features -- -D warnings`: aprovado;
- `cargo test -- --test-threads=1`: 148 testes aprovados, nenhum falhou e um teste permaneceu ignorado na suite regular;
- `cargo test renders_html_to_a_real_pdf -- --ignored --test-threads=1`: teste real de PDF aprovado separadamente com Chromium;
- `cargo llvm-cov --workspace --summary-only`: 83,15% das linhas, 80,57% das regioes e 71,48% das funcoes.

## Revisao de texto aplicada a v09 (2026-08-09)

- Redacao de "diario da alteracao" removida do resumo, do abstract, de 4.4.3, de 6.1 e de 7; descricao passou a refletir o estado final do sistema.
- Subsecao "Migracao monetaria" removida de 4.4 (fundida ao enunciado de 4.4.1) e 4.4.x reenumerada de forma continua (1 a 5).
- Negrito removido de "Lucro bruto estimado" em todas as ocorrencias, mantendo apenas o destaque de titulo.
- Monetario descrito como "representado em centavos inteiros e descontos em pontos-base, com leitura compativel de versoes anteriores", sem termos de cronologia da mudanca.
- Tabelas do bloco de impressao rebaixadas para 10 pt (texto normal segue em 12 pt) com line-height 1,35.
- PDF final revisado com 18 paginas A4.

## Escopo adicional da v09

- migracao de valores monetarios legados para centavos inteiros;
- descontos representados em pontos-base;
- validacao de limites monetarios antes do IPC;
- compatibilidade de leitura com backups nos formatos 1, 2 e 3;
- exportacao de novos backups no formato 3;
- reconciliacao exata do rateio de descontos no relatorio avancado;
- mesma formula de lucro bruto estimado no painel, relatorio avancado, CSV e PDF.

## Documento

- HTML fonte: `artigo-tcc2-v09.html`;
- PDF: `final_modelo_opet_tcc_v09.pdf`;
- 18 paginas em papel A4, abaixo do limite de 20 paginas;
- Figuras 3 e 4 obtidas de capturas do aplicativo;
- Arial regular e negrito incorporadas;
- banner e duas capturas do aplicativo incorporados ao PDF;
- texto das 18 paginas extraido sem caracteres invalidos ou termos de cronologia de versao;
- verificacao estrutural das 18 paginas: nenhuma pagina vazia, texto dentro dos limites da area util e fontes incorporadas.

## Rastreabilidade pendente

O repositorio e publico, mas este estado de trabalho ainda aguarda publicacao sob commit e tag especificos. O hash deve ser acrescentado a esta evidencia e ao TCC quando a publicacao for autorizada.
