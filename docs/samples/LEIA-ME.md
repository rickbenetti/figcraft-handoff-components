# Amostras e fixtures

## Regra permanente

Todo `figcraft-cli run` leva `--out` apontando para uma pasta temporária (o scratchpad). Nunca
`--in` sozinho e nunca sobre a pasta onde o arquivo original mora.

Motivo: sem `--out`, um script que altera o documento grava de volta no arquivo de `--in`. Com
`--out`, o `--in` fica intacto. Só os scripts que não mudam nada (leituras) deixam o arquivo como
estava, e mesmo esses seguem a regra.

## Aviso: resíduos em ../../tests/fixtures/fixture-copia-2.figcraft

`../../tests/fixtures/fixture-copia-2.figcraft` contém resíduos: a instância **36** (com o filho **37**) e o texto **38**. Foram gravados pela execução que o usuário fez no app às 20:42. **Não apague.**

## Arquivos

- `../../tests/fixtures/fixture-copia.figcraft`: cópia do Handoff Components com a coleção Tokens e o "Botão de teste".
- `../../tests/fixtures/fixture-copia-2.figcraft`: mais as coleções Semantico e Lixo (com resíduos, veja acima).
- `../../tests/fixtures/fixture-carga-v2.figcraft`: a carga de referência: Primitivos (400), 150 aliases em Semantico, componente 39 "Card de
  carga" (155 camadas) e instância 194.

## Outros arquivos

- `../i18n-strings.md`: a tabela de textos do plugin (chave | EN | PT), gerada de `src/i18n.rs`.
- `relatorio-v5-*.png`: relatórios do `gerar` 0.5.0 (id 32 em en e pt; id 3 no fixture-carga-v2 e na cópia do fixture-copia-2; id 194).
- `relatorio-v6-*.png`: relatórios do `gerar` 0.6.0 (id 32 em en e pt; id 3 na cópia do fixture-copia-2).
- `relatorio-v61-*.png`: relatórios do `gerar` 0.6.1 (id 32 em en e pt; id 3 na cópia do fixture-copia-2).
- `relatorio-v7-a-pt-azul.png` e `handoff-*-example.json`: o `generate` 0.7.0 depois de `settings` (pt, linha #3B82F6, 1.2.0, 2026-11-01, Aprovado) e o formato do que fica no pluginData.
- `relatorio-v8-id32.png` e `relatorio-v8-id3.png`: o relatório do 0.8.0 (spec à esquerda, palco à direita; id 3 na cópia do fixture-copia-2).
- `relatorio-v82-id32.png` e `relatorio-v82-id194.png` (só o topo): o relatório do 0.8.1, em blocos por camada, com as seções Anatomy e Spacing.
- `fixture-carga-v2 copy.figcraft`: cópia feita fora destes testes (não é gerada por eles); foi commitada por engano numa rodada anterior e o app a modificou depois.

## Fixtures

Os fixtures de teste (`fixture-copia`, `fixture-copia-2` e `fixture-carga-v2`) moram em `tests/fixtures/` (na raiz do repositório de plugins). Os nomes e o conteúdo são os de antes. A demo é o **DS Base Demo**, em `demo/` na raiz do repositório (veja `demo/README.md`).
- `relatorio-v90-toast.png`: o 0.9.0 (Properties, eixos de variante, variante padrão). `component-properties-probe.json`: amostra de `node.get` e `component.properties`.

## Rede de regressão

`scripts/regress.sh` roda o `generate` (headless, num diretório temporário, nunca sobre os fixtures) em 9 casos: id 32 e
id 3 do `fixture-copia-2`, id 3 e id 194 do `fixture-carga-v2`, e Button, Card, Accordion, Code e Icon do DS Base Demo
(`demo/ds-base-demo.figcraft`; os componentes são achados pelo NOME a cada execução, nunca por id fixo). Compara com
`tests/baseline.json`: linha de resumo, linhas auditadas, seções, instâncias, Anatomy, deltas e fuel (tolerância de 15%).
Sai com 1 e lista as diferenças. `scripts/regress.sh --update` regrava o baseline a partir do build atual (faça isso só
quando a mudança de dados for intencional). Precisa do plugin compilado em `dist` (gerado por `scripts/build.sh`) e do
`figcraft-cli` no PATH. `scripts/regress.sh --private` roda só os casos de `tests/private/` (pasta ignorada pelo git,
para arquivos que não podem ser publicados); sem a pasta, avisa e sai sem erro.
