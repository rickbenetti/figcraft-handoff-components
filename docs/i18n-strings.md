# Handoff Components: strings

Generated from `src/i18n.rs` (the test `i18n::tests::strings_doc_is_current` checks it; run it with
`I18N_WRITE=1` to rewrite this file). Placeholders are `{name}`. The language is the `lang`
parameter of the `gerar` command (`en` by default, `pt` or `pt-BR` for Portuguese).

| key | EN | PT |
|---|---|---|
| FrameName | Report — {name} | Relatório — {name} |
| Anatomy | Anatomy | Anatomia |
| LayerTitle | Component | Componente |
| LayerSubtitle | Version · date · status | Versão · data · status |
| LayerStage | Stage | Palco |
| LayerMeasure | Measure {value} | Medida {value} |
| LayerValue | Value {value} | Valor {value} |
| LayerNote | Note | Nota |
| LayerSummary | Summary | Resumo |
| LayerMore | More | Mais |
| LayerAnatomyRow | Anatomy layout | Layout da anatomia |
| LayerMarker | Marker {n} | Marcador {n} |
| LayerLeader | Leader {n} | Guia {n} |
| LayerWarning | Warning | Aviso |
| PropRadius | Radius | Raio |
| PropGap | Gap | Espaçamento |
| PropPadH | Horizontal padding | Padding horizontal |
| PropPadV | Vertical padding | Padding vertical |
| PropFill | Fill | Preenchimento |
| PropStroke | Stroke | Contorno |
| Summary | {bound} bound · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties | {bound} vinculadas · {unbound} sem token · {missing} ausentes · {suspect} suspeitas · de {total} propriedades |
| SummaryChecks | {bound} bound ({check} to check) · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties | {bound} vinculadas ({check} a verificar) · {unbound} sem token · {missing} ausentes · {suspect} suspeitas · de {total} propriedades |
| SummaryStyles | {bound} bound ({vars} variables, {styles} styles) · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties | = EN |
| SummaryStylesChecks | {bound} bound ({vars} variables, {styles} styles; {check} to check) · {unbound} no token · {missing} missing · {suspect} suspect · of {total} properties | = EN |
| PropTypography | Typography | = EN |
| LegendStyle | style | = EN |
| LegendNoStyle | no style | = EN |
| Collision | Warning: {n} bindings with an incompatible type. This component probably came from another file; numeric tokens may be wrong. | Atenção: {n} vínculos com tipo incompatível. Este componente provavelmente veio de outro arquivo; tokens numéricos podem estar errados. |
| OverrideNote | Instance with overrides: {list}; generated from the component | Instância com overrides: {list}; gerado do componente |
| Variants | {n} variants; first one shown | {n} variantes; mostrada a primeira |
| NoteExport1x | This report is taller than 8,000 px, so its PNG export is set to 1x (at 2x it would pass 16,384 px). | Este relatório tem mais de 8.000 px de altura, então a exportação em PNG está em 1x (em 2x passaria de 16.384 px). |
| InstanceOf | Instance of {name} | Instância de {name} |
| LayerKind | Layer kind | Tipo da camada |
| Unnumbered | +{n} layers without a number | +{n} camadas sem número |
| MoreLines | +{n} lines not shown | +{n} linhas não exibidas |
| NoteLineNoCoords | measure.padding line {i} has no coordinates | a linha {i} do measure.padding não tem coordenadas |
| NoteLineOblique | measure.padding line {i} ("{label}") is neither horizontal nor vertical | a linha {i} ("{label}") do measure.padding não é horizontal nem vertical |
| Done | Report generated: {name} | Relatório gerado: {name} |
| OvCorners | corners | cantos |
| OvPadding | padding | padding |
| OvSize | size | tamanho |
| OvPosition | position | posição |
| OvStrokes | strokes | contornos |
| OvTextStyle | text style | estilo de texto |
| OvFills | fills | preenchimentos |
| OvEffects | effects | efeitos |
| LayerHeaderBlock | Header | = EN |
| LayerAnatomySection | Anatomy section | = EN |
| LayerDivider | Divider | = EN |
| LayerLegend | Legend | = EN |
| LayerGroup | Legend: {name} | = EN |
| LayerBadge | Badge {n} | = EN |
| LayerChip | Token chip | = EN |
| LegendToken | token | = EN |
| LegendCheck | token (check) | = EN |
| LegendSuspect | suspect | = EN |
| LegendMissing | missing | = EN |
| LegendNone | no token | = EN |
| SpacingTitle | Spacing | = EN |
| LayerSpacingSection | Spacing section | = EN |
| LayerBody | Properties | = EN |
| LayerBlockHeader | Block header | = EN |
| SpDirection | Direction | = EN |
| SpAlignment | Alignment | = EN |
| DirHorizontal | Horizontal | = EN |
| DirVertical | Vertical | = EN |
| DirGrid | Grid | = EN |
| PropertiesTitle | Properties | = EN |
| LayerProperties | Properties section | = EN |
| LayerPropertyRow | Property: {name} | = EN |
| LayerAxis | Axis: {name} | = EN |
| LayerAxisRow | Variant: {name} | = EN |
| LayerAxisNote | Note | = EN |
| TypeVariant | Variant | = EN |
| TypeBoolean | Boolean | = EN |
| TypeText | Text | = EN |
| TypeInstance | Instance | = EN |
| ValueTrue | True | = EN |
| ValueFalse | False | = EN |
| LegendDefault | Default | = EN |
| DefaultLine | Default: {pairs} ({source}) | = EN |
| SourceSaved | saved | = EN |
| SourceFirst | first variant in the set | = EN |
| NotDefined | Not defined: {list} | = EN |
| MoreVariants | +{n} variants not shown | = EN |
| ErrNoVariants | This component has no variants, so it has no default variant | = EN |
| ErrDefaultMalformed | "{text}" is not Property=Option (for example Size=Small, State=Enabled) | = EN |
| ErrDefaultProp | "{prop}" is not a variant property. Valid: {valid} | = EN |
| ErrDefaultOption | "{value}" is not an option of {prop}. Valid: {valid} | = EN |
| ErrDefaultNoVariant | No variant is named "{name}" | = EN |
| ErrBadStage | stage must be light, dark or auto, not "{value}" | = EN |
| SpHResizing | Horizontal resizing | = EN |
| SpVResizing | Vertical resizing | = EN |
| ResizeHug | Hug | = EN |
| ResizeFixed | Fixed | = EN |
| ResizeFill | Fill | = EN |
| AlignTop | Top | = EN |
| AlignMiddle | Middle | = EN |
| AlignBottom | Bottom | = EN |
| AlignLeft | Left | = EN |
| AlignCenter | Center | = EN |
| AlignRight | Right | = EN |
| AlignSpaceBetween | Space between | = EN |
| AlignSpaceAround | Space around | = EN |
| AlignSpaceEvenly | Space evenly | = EN |
| SourceComponent | component default | = EN |
| DeltaWidth | Width | = EN |
| DeltaHeight | Height | = EN |
| DeltaStrokeWeight | Stroke weight | = EN |
| DeltaOpacity | Opacity | = EN |
| DeltaEffects | Effects | = EN |
| DeltaContent | Text | = EN |
| DeltaHidden | hidden | = EN |
| DeltaVisible | visible | = EN |
| DeltaMore | +{n} more differences | = EN |
| DeltaNone | No differences from default | = EN |
| DeltaEffectOnly | No readable differences (visual effect only) | = EN |
| LayerDeltas | Differences | = EN |
| ErrNoSelection | Select a component or an instance | Selecione um componente ou uma instância |
| ErrMetaSelection | Select the component or instance whose version, date or status you are setting | Selecione o componente ou a instância cuja versão, data ou status você está definindo |
| ErrBadLang | language must be en or pt, not "{value}" | o idioma deve ser en ou pt, não "{value}" |
| ErrBadRerun | on rerun must be replace or new, not "{value}" | ao repetir deve ser replace ou new, não "{value}" |
| ErrBadColour | "{value}" is not a hex colour like #FF4D6D | "{value}" não é uma cor hexadecimal como #FF4D6D |
| SettingsSaved | Settings saved | Configurações salvas |
| SettingsKept | Nothing to change | Nada a alterar |
| ErrNotComponent | layer {id} is a {kind}, not a component or an instance | a camada {id} é {kind}, não um componente nem uma instância |
| ErrNoMain | instance {id} has no main component | a instância {id} não tem componente principal |
| ErrEmptySet | component set {id} has no variants | o conjunto de componentes {id} não tem variantes |
| ErrCall | {cmd}: {reason} | {cmd}: {reason} |
| ErrNoId | {what}: no id in the reply | {what}: sem id na resposta |
| ErrNoPage | page.list: no page | page.list: nenhuma página |
| ErrNotList | page.list: the reply isn't a list | page.list: a resposta não é uma lista |
| ErrParent | instance.place put the instance in {parent}, not in the stage {stage} | instance.place colocou a instância em {parent}, não no palco {stage} |
