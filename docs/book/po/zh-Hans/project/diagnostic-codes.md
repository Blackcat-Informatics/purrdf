<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/project/diagnostic-codes.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。诊断代码本身永不翻译。
-->

# 诊断代码参考

PurRDF 报告的每一次失败都在其 `RdfDiagnostic`（`severity`、`code`、`message`、
`detail`、`location`）上携带一个稳定的、机器可读的 `code`。代码是契约：测试、SARIF
发射器、CLI、Python 的 `ValueError` 文本与下游匹配器都以它为键，而 `message` 是
可能改变措辞的自由文本。本页按家族列出各代码、每个代码所指的失败，以及调用方能做
什么。这个集合是从源码树中的构造位置枚举出来的，不是凭记忆写下的；若发现某个
代码不在此处，以源码为准，本页已过期。

代码为 kebab-case，以拥有它的家族作前缀。它们永不翻译，调用方应比较整个字符串而非
前缀。

## `iri-*`——IRI（国际化资源标识符）解析与基础 IRI 解析（`purrdf-iri`）

`IriError::diagnostic_code` 是整个工作区中这些字符串的唯一所有者；每个编解码器、
SPARQL、ShEx 与 SHACL 都经由它报告 IRI 失败。两个与基础 IRI 相关的代码是有意区分
的，因为它们的补救方式不同：一个靠提供基础 IRI 即可修复，另一个不行。

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `iri-empty` | 在需要非空 IRI/URI 的位置上字符串为空。 | 提供一个非空 IRI。 |
| `iri-missing-scheme` | 字符串没有 scheme，因此不可能是绝对 IRI。 | 以带 scheme 的绝对形式书写该 IRI。 |
| `iri-bad-scheme` | scheme 格式错误（必须以字母开头，且只含字母、数字、`+`、`-` 与 `.`）。 | 更正 scheme。 |
| `iri-bad-percent-encoding` | 报告的字节偏移处有一个未跟随两个十六进制数字的 `%`。 | 对 `%` 本身做百分号编码，或补全转义。 |
| `iri-disallowed-char` | 报告的字节偏移处出现了 IRI 语法不允许的字符。 | 对该字符做百分号编码或将其删除。 |
| `iri-bad-authority` | authority 组件（`//host:port`）格式错误。 | 更正主机或端口。 |
| `iri-non-absolute-base` | 提供用于解析的基础 IRI 本身不是绝对的。 | 提供一个带 scheme 的基础 IRI，例如 `http://example.org/dir/`。 |
| `iri-relative-no-base` | 遇到相对 IRI 引用时作用域内没有基础 IRI：没有文档内指令，没有调用方提供的基础 IRI，也没有检索 IRI。 | 为文档添加基础 IRI（Turtle 族语法中的 `@base`/`BASE`，RDF/XML 中的 `xml:base`，JSON-LD 中的 `@context.@base`），或向 API 传入一个基础 IRI。 |
| `iri-not-absolute-by-grammar` | 该引用不是绝对的，而所用语法根本不允许相对引用（N-Triples、N-Quads），因此任何基础 IRI 都无法适用。 | 以绝对形式书写该 IRI；提供基础 IRI 无济于事。 |

## `native-codec-*`——文本与 XML 编解码器（`purrdf-rdf`）

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `native-codec-parse` | 编解码器（Turtle 族、RDF/XML、TriX、HexTuples）无法解析输入；位置给出行与列。词项嵌套超过解析器深度上限也以此代码报告。 | 在报告的位置修正文档。 |
| `native-codec-utf8` | 输入字节不是合法的 UTF-8。 | 将文档重新编码为 UTF-8。 |
| `native-codec-panic` | 编解码器在解析时发生 panic 并被防护捕获。这是 PurRDF 的缺陷，绝不是输入的问题。 | 连同触发它的输入一起报告。 |
| `native-codec-read` | 经由流式读取器读取 RDF 源时发生 IO 错误。 | 检查源流或文件。 |
| `native-codec-write` | 写出序列化结果时发生 IO 错误。 | 检查目标位置。 |
| `native-codec-serialize` | 目标格式无法表示该数据集——例如单图格式中的命名图——或写入器失败。 | 选择能承载该构造的格式，或经由损失台账通道序列化。 |
| `native-codec-replay` | 把已解析的数据集重放进事件汇时失败，因为该汇返回了错误。 | 汇自身的错误在消息中给出。 |
| `native-codec-unsupported-format` | 该媒体类型或格式标识符不对应任何编解码器。 | 使用受支持的媒体类型或格式 id 之一。 |
| `native-codec-datatype-not-iri` | 某字面量的数据类型词项不是 IRI。 | 为该字面量指定 IRI 数据类型。 |
| `native-codec-direction-without-language` | 在没有语言标签的字面量上给出了基础方向。 | 添加语言标签，或去掉方向。 |
| `native-codec-invalid-direction` | 给出了 `ltr` 或 `rtl` 之外的字面量基础方向。 | 使用 `ltr` 或 `rtl`。 |
| `native-codec-iri-missing-value` | 某个 IRI 词项事件携带了空值。 | 提供该 IRI。 |
| `native-codec-missing-reifier-binding` | 某三元组项引用了一个没有绑定的具体化节点（reifier）。 | 先绑定具体化节点，再引用它。 |
| `native-codec-predicate-not-iri` | 只允许 IRI 的位置（谓词）上的词项不是 IRI。 | 使用 IRI 谓词。 |
| `native-codec-reifier-not-triple` | 某具体化节点绑定的不是三元组项。 | 把具体化节点绑定到三元组项。 |
| `native-codec-term-out-of-range` | 事件流中的某个词项 id 超出了该流所引入的范围。 | 产生该流的一方不一致；重新生成。 |
| `native-codec-unbound-triple-term` | 某三元组项既未命名其组成部分，也未命名具体化节点。 | 为该三元组项给出组成部分或具体化节点。 |

后九个出现在编解码器通道消费的是词项事件而非文本时——即一个 GTS 图经由编解码器接口
被解析——它们与下文的 `gts-*` 和 `rdf-ir-*` 代码相互对应。

## `native-jsonld-*` 与 `jsonld-*`——JSON-LD 与 YAML-LD

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `native-jsonld-decode` | JSON-LD 或 YAML-LD 输入在表层格式错误：非法的 JSON/YAML、编码错误，或超出物化载体的字节预算。 | 修正表层语法，或缩减文档。 |
| `native-jsonld-parse` | 输入是格式良好的 JSON-LD，但无法映射到 RDF。 | 更正 JSON-LD 结构。 |
| `jsonld-json-input` | 严格 JSON 读取器拒绝了输入，例如重复的对象成员。 | 删除重复项或更正 JSON。 |
| `jsonld-context-invalid` | 某上下文文档，或严格的带版本号选项文档，是无效的。 | 更正上下文或选项文档。 |
| `jsonld-context-limit` | 超出了上下文处理的某项上限：加载字节数、工作量、定义复杂度，或离线上下文注册表大小。 | 缩减上下文，或提高选项文档所声明的上限。 |
| `jsonld-derived-invalid` | 确定性的数据集 IRI `derived` 模式无法派生前缀（无效的 IRI，或空映射）。 | 更正将据以派生前缀的 IRI。 |
| `jsonld-derived-limit` | 超出了派生上下文的工作量或字节上限。 | 缩减数据集的 IRI 词汇表，或提高所声明的上限。 |
| `jsonld-options-unused` | 为非 JSON-LD/YAML-LD 的格式提供了 JSON-LD 序列化选项。 | 去掉这些选项，或序列化为 JSON-LD/YAML-LD。 |

## `cdt-*`——SEP-0009 复合字面量

`cdt:List` 或 `cdt:Map` 的词法形式在其所在文档的作用域内指称空节点，因此一个无法
解析的形式会让该作用域变得未定义，于是整个文档被拒绝，而不是把该字面量当作不透明值
保留。

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `cdt-literal-malformed` | 某复合字面量的词法形式无法解析。 | 更正该字面量。 |
| `cdt-literal-scan-disagreement` | 有界词法扫描器与完整解析器对该字面量的判断不一致。 | 这是 PurRDF 的缺陷；连同该字面量一起报告。 |

## `rdf-ir-*`——数据集结构（`purrdf-core` 冻结与 GTS 导入）

`RdfDatasetBuilder::freeze()` 在冻结之前校验数据集的结构；GTS 导入汇对于词项不构成
良构数据集的容器报告同一家族。

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `rdf-ir-term-out-of-range` | 某四元组引用了构建器从未驻留的 `TermId`。 | 推入四元组之前先驻留该词项。 |
| `rdf-ir-predicate-not-iri` | 某四元组的谓词不是 IRI。 | 使用 IRI 谓词。 |
| `rdf-ir-literal-subject` | 字面量出现在主语位置。 | RDF 不允许字面量作主语；重构该陈述。 |
| `rdf-ir-triple-subject` | 三元组项出现在主语位置。 | RDF 1.2 只允许三元组项出现在宾语位置；使用具体化节点。 |
| `rdf-ir-graph-name-invalid` | 某图名是字面量或三元组项。 | 图名必须是 IRI 或空节点。 |
| `rdf-ir-reifier-not-triple` | 某具体化节点绑定指向的不是三元组项。 | 把具体化节点绑定到三元组项。 |
| `rdf-ir-triple-cycle` | 某三元组项直接或经由嵌套包含了自身。 | 消除环。 |
| `rdf-ir-triple-nesting-limit` | 三元组项嵌套超过了构建器的深度上限。 | 展平嵌套。 |
| `rdf-ir-dangling-term-ref` | 某 GTS 角色引用了没有任何词项事件引入的词项 id。 | 容器不一致；重新生成。 |
| `rdf-ir-gts-fold-diagnostic` | GTS 折叠报告了一条诊断，经由导入浮现。 | 折叠诊断自身的代码与详情在消息中给出。 |
| `rdf-ir-iri-missing-value` | 某个导入的 IRI 词项值为空。 | 提供该 IRI。 |
| `rdf-ir-literal-datatype-not-iri` | 某个导入的字面量的数据类型解析为非 IRI。 | 为该字面量指定 IRI 数据类型。 |
| `rdf-ir-missing-reifier-binding` | 某个导入的三元组项引用了没有记录绑定的具体化节点。 | 在容器中绑定该具体化节点。 |
| `rdf-ir-term-nesting-limit` | 导入的三元组项嵌套超过了深度上限。 | 展平嵌套。 |
| `rdf-ir-unbound-triple-term` | 某个导入的三元组项既未命名其组成部分，也未命名具体化节点。 | 为该三元组项给出组成部分或具体化节点。 |

## `gts-*` 与 `rdf-*`——GTS 图的解析、验证与写入

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `gts-term-out-of-range` | 某 GTS 词项 id 超出了该图的范围。 | 容器不一致；重新生成。 |
| `gts-iri-missing-value` | 某 GTS IRI 词项值为空。 | 提供该 IRI。 |
| `gts-predicate-not-iri` | 某 GTS 谓词词项不是 IRI。 | 使用 IRI 谓词。 |
| `gts-literal-datatype-not-iri` | 某 GTS 字面量的数据类型未解析为 IRI。 | 为该字面量指定 IRI 数据类型。 |
| `gts-direction-without-language` | 某 GTS 字面量携带基础方向但没有语言标签。 | 添加语言标签，或去掉方向。 |
| `gts-invalid-direction` | 某 GTS 字面量的基础方向既不是 `ltr` 也不是 `rtl`。 | 使用 `ltr` 或 `rtl`。 |
| `gts-missing-reifier-binding` | 某 GTS 三元组项引用了该图未绑定的具体化节点。 | 在容器中绑定该具体化节点。 |
| `gts-unbound-triple-term` | 某 GTS 三元组项既未命名自身的组成部分，也未命名具体化节点。 | 为该三元组项给出组成部分或具体化节点。 |
| `gts-self-reaching-term` | 某 GTS 词项经由自身解析，因此对其组成部分的任何遍历都无法终止。 | 消除环。 |
| `gts-term-nesting-limit` | GTS 词项嵌套超过了深度上限。 | 展平嵌套。 |
| `gts-fold-diagnostic` | GTS 折叠报告了一条或多条诊断。 | 查看详情中列出的折叠诊断。 |
| `gts-verify-digest-inclusion` | 有内容寻址的词项未包含在已验证的链中。 | 该容器的链未覆盖其内容；不要信任它。 |
| `gts-verify-signature` | COSE 签名验证失败。 | 检查签名密钥与容器的完整性。 |
| `gts-writer-codec` | GTS 写入器的编解码器在写入时报告了错误。 | 编解码器自身的错误在消息中给出。 |
| `rdf-graph-name-not-node` | 构建 GTS 图时，某命名图的名称不是 IRI 或空节点。 | 使用 IRI 或空节点作为图名。 |
| `rdf-reifier-not-node` | 构建 GTS 图时，某 RDF 1.2 具体化节点不是 IRI 或空节点。 | 使用 IRI 或空节点作为具体化节点。 |
| `rdf-term-nesting-limit` | 构建 GTS 图时 RDF 词项嵌套超过了深度上限。 | 展平嵌套。 |

## `native-sparql-*`——SPARQL 引擎边界（`purrdf-sparql-eval`）

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `native-sparql-query-parse` | 查询文本无法按 SPARQL 1.1/1.2 语法解析（包括格式错误的 `VERSION` 声明；格式正确但无法识别的版本属于 `native-sparql-unsupported`）。 | 在所报告的位置修正查询。 |
| `native-sparql-update-parse` | 更新请求无法解析。 | 在所报告的位置修正更新。 |
| `native-sparql-query-explain` | 在 `--explain` 下求值失败；求值器的错误包含在消息中。 | 处理底层的求值错误。 |
| `native-sparql-query-eval` | 查询求值失败，且没有更具体的代码对该错误分类；求值器的错误包含在消息中。 | 处理底层的求值错误。 |
| `native-sparql-update-eval` | 更新求值失败，且没有更具体的代码对该错误分类；求值器的错误包含在消息中。 | 处理底层的求值错误。 |
| `native-sparql-algebra` | 查询的代数在求值之前被拒绝：`Query::validate` 拒绝了它，或者某个预备好的计划需要针对本次求值提供的注册表重新规划。 | 按消息所说修正查询，或在执行之前预备改动后的代数。 |
| `native-sparql-construct` | 类型化的图构建收到的查询不是 `CONSTRUCT`。 | 用 `CONSTRUCT` 查询构建图。 |
| `native-sparql-execution-parameter` | 预备好的执行收到了它未声明的参数名或槽位。 | 只绑定该执行所声明的参数；消息中列出了它们。 |
| `native-sparql-relation-incomplete` | 某个属性函数关系只用其部分数据作答，而所在的入口没有用于标注这一不完整应答的证据槽（不受管控的查询通道，以及 UPDATE）。消息会指名该关系及其自身给出的原因。 | 在受 governor 管控的入口上运行查询——其回执携带该关系的见证——或等待该关系完整。 |
| `native-sparql-property-function` | 属性函数（property function）扩展点拒绝了该查询：某个已声明命名空间下的谓词没有注册，某个调用点的元数与该关系不匹配，没有任何全序能用于某条链，或者一个预备好的计划正在一个与其预备时不同的注册表下求值。 | 注册该关系、更正元数，或在同一个注册表下预备并求值。 |
| `native-sparql-aggregate-function` | 自定义聚合扩展点拒绝了该查询：某个 `AGG(<iri>, …)` 没有指向任何已注册的聚合，或者一个预备好的计划正在另一个聚合注册表下求值。 | 注册该聚合，或在同一个注册表下预备并求值。 |
| `native-sparql-custom-function` | 某个函数或聚合 IRI 没有解析到任何已注册的自定义函数、原生函数或 XSD 构造函数。 | 在该 IRI 下注册函数，或使用原生函数。 |
| `native-sparql-quoted-triple-term-variable` | 在基本图模式或属性路径中，某个变量占据了引用三元组项的一个组成部分；结构化的三元组项匹配不在支持范围之内。 | 把三元组项作为整体绑定，或通过具体化节点接口匹配其组成部分。 |
| `native-sparql-heldin-unconfigured` | 调用 `heldIn` 时没有调用方提供的 standpoint（立场）谓词配置。 | 在使用 `heldIn` 之前先配置 standpoint 谓词。 |
| `native-sparql-evaluation-stack-exhausted` | 请求的嵌套深度超出了求值它的线程栈所能容纳的范围：在递归遍历开始之前，求值器会用剩余的栈空间衡量计划的整体高度（关联替换在其所做副本的每一层也会检查），并在耗尽之前拒绝，而不是让进程中止或让 wasm 实例陷入 trap。解析器已接受该请求，在更大的栈上同一请求可以得到应答。 | 在以更大的栈创建的原生线程上求值，或减少请求的嵌套深度。在 wasm 中，两条通道运行在大小相同的栈上——异步作业的栈区域与模块自身的影子栈完全一样大——因此在那里的补救办法是减少嵌套深度，消息也不会给出其他办法。 |
| `native-sparql-unsupported` | 请求本身是良构的，但使用了本引擎拒绝按原样求值的内容——例如没有任何解为其指明端点的 `SERVICE ?e`、（不写 `SILENT` 时）绑定到字面量的 `?e`，或无法识别的 `VERSION`。消息会指出具体是什么，并在可行时给出改写方式。Cloudflare 适配器以 `400` 应答。 | 按消息所述改写请求。 |
| `native-sparql-host-stack-exhausted` | 仅限 wasm：请求的嵌套深度超出了 JavaScript 引擎自身调用栈所能容纳的范围。PurRDF 为求值每个请求保留该栈中固定的 640 KiB 预算——图模式最多嵌套 284 层，整棵树最多 2 048 层外加每个嵌套组八层，表达式与路径最多 2 304 层——而 V8 为同步通道和异步作业分配的栈相同，因此超出预算的计划会在运行之前被拒绝，两条通道一致。解析不消耗该栈的任何部分。Cloudflare 适配器以 `400` 应答。 | 减少请求的嵌套深度。 |
| `native-sparql-bnode-mint-prefix` | 选项中提供的空节点生成前缀无效。 | 提供一个有效的前缀。 |
| `native-sparql-service-denied` | 已安装的 `ServiceCatalog` 未向请求指名的某个 `SERVICE` 端点授予所需的能力——该端点没有 profile，或其 profile 未授予 `query` 或 `network`——因此没有联系任何端点。在 `SERVICE SILENT` 下该子句则是连接的单位元，并记录为一次被静默的 `denied` 调用。Cloudflare 适配器以 `403` 应答。 | 在目录中为该端点授予该能力，或去掉 `SERVICE`。 |
| `native-sparql-service-host-denied` | 宿主自己的服务解析器依据其自身策略拒绝了某个 `SERVICE` 请求，且没有指名任何目录能力——例如某个 JavaScript `resolveService` 应答了 `{ kind: "denied" }`——因此没有联系任何端点。在 `SERVICE SILENT` 下该子句则是连接的单位元，并记录为一次被静默的 `host-denied` 调用。Cloudflare 适配器以 `403` 应答。 | 在宿主策略中允许该请求，或去掉 `SERVICE`。 |
| `native-sparql-service-failed` | 已联系某个 `SERVICE` 端点，但它没有给出可用的应答：传输失败、HTTP 错误状态、重定向、解析器自身的超时，或无法解码的响应体。在 `SERVICE SILENT` 下该子句则是连接的单位元，并记录为一次被静默的调用。Cloudflare 适配器以 `502` 应答。 | 让该端点可达，或写作 `SERVICE SILENT`。 |
| `native-sparql-service-unconfigured` | 没有任何远程查询来源能到达请求指名的某个 `SERVICE` 端点：要么没有配置来源，要么已配置的来源应答说它所持有的内容都到达不了该端点。在 `SERVICE SILENT` 下该子句则是连接的单位元，并记录为一次被静默的 `unconfigured` 调用。Cloudflare 适配器以 `500` 应答。 | 配置一个能到达该端点的来源，或去掉 `SERVICE`。 |
| `native-sparql-load-no-resolver` | 请求了 `LOAD <iri>`，但没有提供 `GraphResolver` 宿主扩展点。`LOAD SILENT` 则会成功返回、不加载任何内容，并记录为一次被静默的 `unconfigured` 调用。 | 注入一个解析器，或去掉该 `LOAD`。 |
| `native-sparql-load-denied` | 已安装的 `ServiceCatalog` 未向 `LOAD` 来源授予所需的能力（`LoadError::Denied`）——该来源或某次重定向指明的位置没有 profile，或其 profile 未授予 `network`——因此没有抓取任何内容。`LOAD SILENT` 则会成功返回、不加载任何内容，并记录为一次被静默的 `denied` 调用。Cloudflare 适配器以 `403` 应答。 | 在目录中为该来源授予 `network`，或去掉 `LOAD`。 |
| `native-sparql-load-host-denied` | 宿主自己的 `LOAD` 解析器依据其自身策略拒绝了该来源，且没有指名任何目录能力（`LoadError::HostDenied`）——例如某个 JavaScript `resolveLoad` 应答了 `{ kind: "denied" }`。`LOAD SILENT` 则会成功返回、不加载任何内容，并记录为一次被静默的 `host-denied` 调用。Cloudflare 适配器以 `403` 应答。 | 在宿主策略中允许该来源，或去掉 `LOAD`。 |
| `native-sparql-load-failed` | `GraphResolver` 无法抓取该文档（`LoadError::Transport`）——例如 wasm 包的异步 `resolveLoad` 应答了 `{ kind: "transport" }`、抓取超出了其目录 profile 的超时，或跟随了五次以上的重定向。`LOAD SILENT` 则会成功返回、不加载任何内容，并记录为一次被静默的调用。Cloudflare 适配器以 `502` 应答。 | 让该来源可达，或写作 `LOAD SILENT`。 |
| `native-sparql-load-decode` | `GraphResolver` 抓取到了文档，但无法解析它（`LoadError::Decode`）——例如 wasm 包的异步 `resolveLoad` 交付的文档所用的媒体类型没有解析器可读，或者字节并不是所指名的语法。`LOAD SILENT` 则会成功返回、不加载任何内容，并记录为一次被静默的调用。Cloudflare 适配器以 `502` 应答。 | 以解析器能够解析的语法提供文档，或写作 `LOAD SILENT`。 |
| `native-sparql-load-stopped` | 在等待 `LOAD` 期间，请求自身的停止信号（取消或截止时间）被触发（`LoadError::Governed`）。受 governor 管控的请求报告的是 governor 触发，而不是这个代码，无论是否写了 `SILENT`。 | 放宽截止时间，或不要取消该作业。 |
| `native-sparql-load-fault` | `GraphResolver` 的处理函数发生故障（`LoadError::Fault`）——例如 wasm 包的异步 `resolveLoad` 抛出异常、返回被拒绝的 Promise、什么也没有应答，或指明了协议未定义的失败种类。消息携带处理函数自己的原话。`LOAD SILENT` 则会成功返回、不加载任何内容，并记录为一次被静默的 `fault` 调用。Cloudflare 适配器以经过清理的 `500` 应答，真正的原话以关联 id 写入其日志。 | 返回一份带媒体类型的文档、一个 `Response`、一个 `Dataset`、一次重定向或一个带类型的失败——绝不要抛出异常。 |
| `native-sparql-cancelled` | wasm 异步通道：一个不受 governor 管控的操作被取消（其 `AbortSignal` 已触发）。受管控的操作则在其结果中报告这次触发。Cloudflare 适配器以 `503` 应答。 | 不要取消该操作。 |
| `native-sparql-deadline` | wasm 异步通道：一个不受 governor 管控的操作超过了截止时间。Cloudflare 适配器以 `503` 应答。 | 放宽截止时间。 |
| `native-sparql-not-acceptable` | wasm 异步通道：一个协商查询的结果——一个带有命名图的图——没有任何 `Accept` 请求头允许的格式能够承载。Cloudflare 适配器以 `406` 应答，并列出能够承载它的格式。 | 接受 TriG、N-Quads 或 JSON-LD。 |
| `native-sparql-update-in-flight` | wasm 异步通道：同一数据集的另一个异步更新已在途，或者数据集在该更新运行期间被修改，因此该更新被拒绝，而不是针对一个它从未见过的状态求值（或提交在其之上）。Cloudflare 适配器以 `409` 应答。 | 等待第一个更新完成，再发送下一个。 |
| `native-sparql-host-fault` | 失败的是宿主，而不是请求，也不是某个端点：某个 `SERVICE` 来源的处理函数在不带 `SILENT` 的子句中发生故障（`EvalError::ServiceHostFault`）——例如某个 wasm `resolveService` 抛出异常、返回被拒绝的 Promise、以不是应答的内容作答，或指明了协议未定义的失败种类（在 `SERVICE SILENT` 下该故障是连接的单位元，并在证据中记录为一次被静默的 `fault`）；或者，在 wasm 异步通道上，某个作业的交付协议被破坏，或其栈帧覆写了其栈区域的金丝雀字。Cloudflare 适配器以经过清理的 `500` 应答，真正的原话以关联 id 写入其日志。 | 修正宿主的处理函数。 |
| `native-sparql-governor-ceiling` | 协议边界：某个确定性的 governor 上限叫停了该操作——上限已达到，或规划器的估算已经超出。操作的结果携带这次触发；这是宿主据以应答它的 problem 的 `code`（Cloudflare 适配器的 `422`）。 | 提高上限，或收窄请求。 |
| `native-sparql-evaluation` | 协议边界：操作的求值失败，且没有更具体的代码；这是宿主据以应答它的 problem 的 `code`。 | 处理底层的求值错误。 |
| `native-sparql-update-bad-destination` | `ADD`/`MOVE`/`COPY`/`LOAD` 的目标是 `NAMED` 或 `ALL`；它必须是 `DEFAULT` 或单个命名的 `GRAPH`。 | 指定单个目标图。 |
| `native-sparql-subst-iri` | 某个替换值不是有效的 IRI。 | 提供一个有效的 IRI。 |
| `native-sparql-subst-triple-predicate` | 某个被替换的引用三元组的谓词不是 IRI。 | 使用 IRI 谓词。 |
| `native-sparql-subst-literal-datatype` | 被替换的字面量的数据类型不是 IRI。 | 使用 IRI 数据类型。 |
| `native-sparql-subst-langtag` | 预绑定字面量的语言标签不是该词法 profile 所接受的。 | 提供一个良构的语言标签。 |

## `reasoning-*`——蕴涵机制下的 SPARQL（`purrdf`）

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `reasoning-closure-relation-witness` | 从推理闭包派生的属性函数关系不能与一次受限 chase 生成了存在性见证的 OWL 直接语义运行相结合：遍历闭包的关系可能返回一个该蕴涵机制的作用域图并不包含的、新生成的空节点。 | 在不生成见证的蕴涵机制（`rdf`、`rdfs`、`owl-rl`、`d`、`rif`、`simple`）下查询，或从本次调用中去掉由数据集派生的关系。 |
| `reasoning-closure-relation-rebuild` | 在推理闭包上重建属性函数关系失败。 | 关系构建器自身的错误在消息中给出。 |

## `statements-*`——陈述元数据摄入（`purrdf-rdf`）

这些代码出现在把文档中的 `rdf:reifies` 与 `owl:Axiom` 陈述读入陈述层时。

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `statements-turtle-parse` | 陈述元数据的 Turtle 无法解析。 | 修正该 Turtle。 |
| `statements-non-iri` | 此处必须是 IRI 的词项不是 IRI。 | 使用 IRI。 |
| `statements-reifies-non-triple` | `rdf:reifies` 的宾语不是三元组项。 | 具体化一个三元组项。 |
| `statements-malformed-axiom` | 某 `owl:Axiom` 缺少其 source、property 或 target。 | 补全该公理。 |
| `statements-conflicting-structural` | 同一主语在某个结构字段上携带了两个不同的值。 | 只保留一个值。 |

## 其他单代码家族

| 代码 | 含义 | 补救 |
| --- | --- | --- |
| `sssom-tsv-parse` | SSSOM TSV 文档格式错误：表头行缺失或不可读、`curie_map` 条目或集合注释格式错误、某行格式错误，或 confidence 非数值；位置给出行号。 | 在报告的行修正该 TSV。 |
| `content-id-scheme` | 某内容 id 的 scheme 前缀无效：为空、非 ASCII，或以十六进制数字结尾（那会使它与 64 个十六进制字符的尾部产生歧义）。 | 选择非空、ASCII 且不以 `0-9`、`a-f` 或 `A-F` 结尾的前缀。 |

## 代码的到达路径

- **Rust**——返回错误上的 `RdfDiagnostic::code`；其 `Display` 形式为
  `<severity> <code>: <message>`。
- **Python**——`ValueError` 的消息就是该 `Display` 形式（例如
  `error native-codec-parse: …`）；IRI 失败渲染为 `<code>: <message>`（例如
  `iri-relative-no-base: …`）。
- **JavaScript**——抛出的 `Error` 消息携带同样的文本。
- **C**——经由 C ABI 返回的错误字符串携带同样的文本。
- **SARIF**——每个结果的 `ruleId` 就是该代码，并在本次运行的规则表中附带一个
  `reportingDescriptor`（[`purrdf-validate`](https://docs.rs/purrdf-validate)）。
