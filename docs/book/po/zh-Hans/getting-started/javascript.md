<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/getting-started/javascript.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。代码块与英文原文逐字节相同。
-->

# 入门：JavaScript / WebAssembly

npm 包
[`@blackcatinformatics/purrdf`](https://www.npmjs.com/package/@blackcatinformatics/purrdf)
是同一个 Rust 引擎编译到 `wasm32` 后，以 [RDF/JS](https://rdf.js.org/) 形态的 API
（`DataFactory`、`DatasetCore`、`Stream`/`Sink`）呈现出来的结果。它在浏览器和 Node
中运行，完全驻留内存。

```sh
npm install @blackcatinformatics/purrdf
```

**译注：中国大陆镜像。**从中国大陆访问 npm 注册表时常有延迟或间歇性不可达。常用的镜像是
`npmmirror.com`（原 cnpm/淘宝源）：`npm config set registry https://registry.npmmirror.com`。
具体配置请参考各镜像站的说明，本文不再重复。

想在安装任何东西之前先试一试？
[RDF-1.2 演练场](https://blackcat-informatics.github.io/purrdf/playground/)
在浏览器中运行的正是这份 wasm 构建——解析、SPARQL、SHACL、序列化，以及 RDF-1.2 图的
规范化/比较，全部在客户端完成，不需要工具链，也不需要服务器。

## 第一个数据集

在做任何事之前先 `await ready()` 一次——它执行一次性的异步 wasm 实例化：

```js
import { ready, DataFactory, Dataset, QueryEngine } from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const f = new DataFactory();
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");

const ds = new Dataset();
ds.add(f.quad(f.namedNode("https://ex/s"), f.namedNode("https://ex/says"), rtl));

const nq = ds.serialize("nquads");           // directions survive the round-trip
const reparsed = Dataset.parse(nq, "nquads");

const engine = new QueryEngine();
const ask = engine.ask(reparsed, "ASK { <https://ex/s> <https://ex/says> ?msg }");
```

## RDF 1.2：其他库尚未覆盖之处

没有哪个现存的 RDF/JS 库承载 RDF 1.2 的**引用三元组项**（quoted-triple term）或
**方向字面量**（directional literal）。PurRDF 的 `DataFactory` 两者都提供：

```js
// A quoted triple, usable as a subject/object (RDF-star / RDF 1.2).
const quoted = f.quotedTriple(
  f.namedNode("https://ex/alice"),
  f.namedNode("https://ex/knows"),
  f.namedNode("https://ex/bob"),
);

// A base-direction literal (rdf:dirLangString).
const hello = f.directionalLiteral("مرحبا", "ar", "rtl");
```

## API 接口

- **`ready(bytesOrUrl?)`**——在做任何事之前 await 一次；它也接受已编译的 `WebAssembly.Module`。
- **`DataFactory`**——`namedNode`、`blankNode`、
  `literal(value, languageOrDatatype?)`、`typedLiteral`、
  `directionalLiteral`、`variable`、`defaultGraph`、`quad`、`quotedTriple`、
  `fromTerm`、`fromQuad`。
- **`Dataset`**（RDF/JS `DatasetCore`）——`Dataset.parse(input, format, base?)`、
  `serialize(format)`、`add`/`delete`/`has`/`match`/`quads`/`size`，以及
  迭代（`for (const quad of dataset)`）。格式：`turtle`、`ntriples`、
  `nquads`、`trig`、`rdfxml`（或其媒体类型）；`serialize` 另外接受
  `jsonld`。
- **图同一性**——`Dataset.canonicalize()` 返回该图在 RDFC-1.0 下的规范、扁平
  N-Quads；`Dataset.isomorphic(other)` 在空节点重命名下判定 RDF 图相等（一个由
  完整 RDFC-1.0 规范化支撑的精确判定器）。
- **图/表格/ Research Object（RO）载体**——`Dataset.project(profile, configJson)` 返回规范的
  USTAR 字节与损失台账 JSON；`Dataset.projectWithAssets("ro-crate-1.3",
  configJson, payloadArchive)` 加入有界的、随附的 RO-Crate 载荷；
  `liftProjection(...)` 为各双向 profile 重建 RDF。参见
  [图、表格与 Research Object 投影](../concepts/projections.md)。
- **SPARQL**——`QueryEngine` 在多次调用之间保持原生计划缓存存活，并暴露带类型的 `select` / `ask` / `construct` / `describe`、原子的 `update`，以及 `queryRaw` 序列化。`Dataset.query(...)` 仍作为兼容用的裸字符串辅助方法保留。每个求值方法都有一个返回 Promise 的孪生方法，接受宿主为 `SERVICE` 与 `LOAD` 提供的处理函数——见[下文](#asynchronous-queries-and-federation)。
- **SHACL**——`shaclValidateToSarif(shapesTtl, dataNt)` 用一份 Turtle 形状图验证一份
  N-Triples 数据图并返回 SARIF 2.1.0 报告；`shaclEntail(shapesTtl, dataNt)` 把
  SHACL-AF `sh:rule` 的推论物化为 N-Triples。
- **`Sink`**——流式消费者（`push(quad)` / `finish() → Dataset`）；
  `datasetToStream` / `streamToDataset` 是异步的 RDF/JS Stream/Sink 辅助方法。

RDF/JS 映射的更多内容见 [JavaScript 中的 RDF/JS](../interop/rdfjs.md)。

## Asynchronous queries and federation

<!-- 此标题保留英文：本书其他页面以 #asynchronous-queries-and-federation 链接到这里，而锚点由标题文字生成。 -->

同步方法是离线通道：它们不安装任何 `SERVICE` 或 `LOAD` 来源，因此非 `SILENT` 的 `SERVICE` 或 `LOAD` 会按名称失败。每个求值方法还有一个返回 Promise 的孪生方法——`QueryEngine` 上的 `queryAsync`、`selectAsync`、`askAsync`、`constructAsync`、`describeAsync`、`queryRawAsync`、`queryRawBytesAsync`、`queryRawWithContextAsync`、`queryGovernedAsync`、`queryEntailmentGovernedAsync`、`updateAsync`、`updateGovernedAsync` 与 `explainQueryAsync`，以及 `Dataset.queryAsync`——此外还有 `queryGovernedNegotiatedAsync`，它把受 governor 管控的查询按 HTTP `Accept` 请求头协商出的格式作为文档返回。孪生方法在调用开始时为数据集拍下快照，并在快照上运行同一个求值器；它以作业的形式执行：宿主应答 `SERVICE` 或 `LOAD` 时作业挂起，求值期间作业把事件循环让出，最终兑现为与其同步孪生方法完全相同的返回值。I/O 由宿主完成，相应的策略也归宿主所有；解析、求值、连接、`SILENT` 语义与结果编码仍由 PurRDF 负责。

EXPLAIN 也是这类求值方法之一：它的计费台账是实际运行查询测得的，而不是根据查询文本预测的。因此，`explainQuery` 会因缺少来源而拒绝 `SERVICE` 查询，而 `explainQueryAsync` 则基于宿主的应答来解释它。它的测量运行与其他作业一样会让出事件循环，并在其 `signal` 触发时停止；它不接受任何上限，因为这次运行只计量、从不设限。

SHACL 验证同样会对 SPARQL 求值：`sh:SPARQLTarget` 查询、SHACL-SPARQL 约束，以及 SHACL-AF 的节点表达式与规则。因此，SHACL 函数也有孪生方法：`shaclValidateToSarifAsync`、`shaclValidateChangesToSarifAsync`、`shaclEntailAsync`，以及 `shaclProductValidateToSarifAsync` 与它的 `Rebuild`、`Expecting` 和 `RebuildExpecting` 形式。每个孪生方法依次接受其同步孪生方法的参数和宿主选项，并返回与同步孪生方法完全相同的结果；被拒绝的预编译形状产物（prepared shapes product）会以同一个 `ShaclProductRefusal` 拒绝。`signal` 既在查询内部轮询，也在焦点节点之间轮询，因此即使一次验证中没有任何 SPARQL，它也会让出事件循环，并在信号触发时停止。SHACL 只允许在不预绑定任何变量的查询（例如 `sh:SPARQLTarget`）中使用 `SERVICE`；使用 `SERVICE` 的约束查询会在加载形状图时被拒绝，两条通道都是如此。

孪生方法运行在 WebAssembly JavaScript Promise Integration（JSPI）之上，它在 Chrome 与 Edge 137+、Firefox 139+、Safari 27、Node 24.20+ 以及 Cloudflare Workers（workerd）中默认启用。`hasAsyncQueries()` 报告当前引擎是否支持 JSPI；在不支持的引擎上，每个孪生方法都会在触及 wasm 之前以同一个错误拒绝，而同步 API 照常工作。

### 应答 `SERVICE`

`resolveService(request, ctx)` 收到的是需要发送的 SPARQL 1.1 Protocol POST 请求：`endpoint`、`queryText`、`contentType`（`application/sparql-query`）、`accept`（`application/sparql-results+json`）、`userAgent`、`timeoutMs`，以及 `headers`——目录 profile 的请求头与凭据，形式为按发送顺序排列的 `[name, value]` 对。`ctx` 携带 `signal`（在取消或到达截止时间时触发）、`remainingDeadlineMs`、`silent` 与 `maxIntermediateCells`。处理函数的应答可以是 SPARQL Results JSON（字节或字符串）、一个 `Response`（非 2xx 状态视为传输失败）、`{ kind: "transport", message }`——`SERVICE SILENT` 会把它吞掉，代之以连接的单位元——或 `{ kind: "denied", message }`，后者即使在 `SILENT` 下也会让查询失败。处理函数若抛出异常、返回被拒绝的 Promise 或返回任何其他值，即视为发生故障；故障即使在 `SERVICE SILENT` 下也会让作业失败，因为它不是应答。`ctx.silent` 仅供参考：处理函数无权自行编造一个空应答。

```js resolve-service-recipe
import { ready, Dataset, QueryEngine } from "@blackcatinformatics/purrdf";

await ready();

// One SERVICE request, sent as a SPARQL 1.1 Protocol POST.
async function resolveService(request, { signal }) {
  try {
    // A Response is an answer as it stands: a 2xx body is read as SPARQL Results
    // JSON, and any other status is a transport failure.
    return await fetch(request.endpoint, {
      method: "POST",
      headers: [
        ["Content-Type", request.contentType], // application/sparql-query
        ["Accept", request.accept], // application/sparql-results+json
        ...request.headers, // the catalog profile's headers, in sending order
      ],
      body: request.queryText,
      signal: AbortSignal.any([signal, AbortSignal.timeout(request.timeoutMs)]),
    });
  } catch (error) {
    // Never rethrow: a throw is a fault, which fails the query even under SERVICE SILENT.
    return { kind: "transport", message: String(error) };
  }
}

const engine = new QueryEngine();
const dataset = Dataset.parse(
  "<https://example.org/a> <https://example.org/p> <https://example.org/o1> .\n",
  "nquads",
);
const { rows } = await engine.selectAsync(
  dataset,
  `SELECT ?s ?x WHERE {
     ?s <https://example.org/p> ?o
     SERVICE <https://remote.example.org/sparql> { ?o <https://example.org/q> ?x }
   }`,
  { resolveService },
);
for (const row of rows) console.log(row.s.value, row.x.value);
```

以 `catalog` 传入的 `ServiceCatalog` 会在调用处理函数之前授权每一个请求（默认拒绝，每个端点一个 profile，另可设一个兜底 profile）；被拒绝的请求即使在 `SERVICE SILENT` 下也会让查询失败。`localServices` 在进程内用一个 `Dataset` 应答指定的端点。`resolveLoad` 以同样的方式应答 `LOAD`：返回一份文档及其媒体类型、一个 `Response`、一个 `Dataset`，或一个带类型的失败；`LOAD SILENT` 会吞掉传输失败，但绝不吞掉拒绝。

在浏览器中，由远程端点的 CORS 策略决定 `fetch` 能否读取其应答：不允许页面所在源的端点会表现为网络错误，上面的处理函数把它报告为传输失败。若某个端点可能不可达、且其结果行可有可无，请写 `SERVICE SILENT`。

变量端点 `SERVICE ?e { … }` 会针对 `?e` 所绑定的每个不同 IRI 各调用一次 `resolveService`，某个端点应答的每一行都带有该 `?e`。到达该子句的每个解都必须绑定 `?e`：可以由同一组中位于其前的模式绑定（三元组模式、`VALUES`、`BIND`，或 `LATERAL { SERVICE ?e { … } }`），也可以由右侧包含该子句的 `OPTIONAL`、`MINUS` 或组连接（group join）的左侧绑定——例如 `?g ex:endpoint ?e OPTIONAL { SERVICE ?e { … } }`，换成 `MINUS` 或 `{ ?g ex:endpoint ?e } { SERVICE ?e { … } }` 亦然。在这些情形下，右侧仍然独立求值，左侧只提供需要询问的端点列表。若某个左侧行的端点没有应答任何结果，该行在 `OPTIONAL` 下保留其原有绑定，在 `MINUS` 下也不会被移除。在 `SERVICE SILENT` 下，失败的端点只贡献一行、且只绑定 `?e`，因此该端点自己的左侧行原样保留、不被扩展，其他端点的结果行也不受影响。若没有任何解为某个子句绑定 `?e`——无处绑定、只在部分左侧解中绑定，或者只在包含该子句的另一个 `OPTIONAL` 或 `MINUS` 右侧、`EXISTS`、`LIMIT`/`OFFSET`、未按 `?e` 分组的聚合或未投影 `?e` 的子 `SELECT` 之外绑定——该子句即被拒绝，无论是否写了 `SILENT`：`SILENT` 容忍的是失败的端点，而不是一个根本没有指明任何端点的查询。错误消息会给出改写方式：在该子句之前绑定 `?e`，例如 `?s ex:endpoint ?e . SERVICE ?e { … }`。

### 让出、取消与并发

- 作业每经过 `yieldEveryPolls` 次 governor 轮询就把事件循环让出一轮（默认 65 536；`0` 表示每次轮询都让出），所用的宏任务原语由 `asyncYieldPrimitive()` 报告。只有求值阶段会让出：冻结数据集与序列化结果都会一次运行到底，`evidence.async` 报告每个阶段的耗时。
- `signal: AbortSignal` 会在作业下一次让出或发出宿主请求时取消它。在受 governor 管控的孪生方法上，`deadlineMs` 包含等待处理函数的时间；一次 governor 触发——包括截止时间与取消——是一个结果，而不是一次 Promise 拒绝。
- 查询读取自己的快照；同一数据集上的异步更新按调用顺序逐个运行，且只有在其运行期间数据集未被修改时才会应用。`configureAsync({ maxConcurrentJobs })` 限定同时在途的作业数（默认 16）。
- 每个作业在自己的栈区域上求值，其大小为 `stackBytes` 字节（默认 2 MiB）；`evidence.async.stackHighWaterBytes` 报告它用到了多深，而嵌套深度超出该区域的请求会以与其同步孪生方法相同的带类型栈拒绝错误失败，并指出补救办法是调大 `stackBytes`。若某个作业触发 trap，或其栈帧越过了区域下方的保护区，该实例即被毒化；任何同步调用中发生的 trap 或 Rust panic 也会同样毒化该实例：此后对本包的每一次调用——包括同步调用，以及对 trap 之前创建的对象的调用——都会抛出错误，只有全新的 JavaScript realm（新的页面、Worker isolate 或进程）才能再次加载本包。

### Cloudflare Workers

`@blackcatinformatics/purrdf/cloudflare` 提供 `createFetchServiceResolver`、`createFetchLoadResolver` 与 `handleSparqlRequest`；后者以一个 `Response` 应答一个 SPARQL 1.1 Protocol 请求：`200` 附带协商出的文档，请求体超出 `maxRequestBytes`（默认 1 MiB——一个查询或更新的文本是一段程序，不是一份负载）时返回 `413`，governor 叫停请求时返回 `422` 或 `503`（绝不会以 `200` 返回部分结果），错误以 `application/problem+json` 给出，`Server-Timing` 取自作业的证据，并在配置后发送 CORS 头。`500` 的 `detail` 只在失败确实是查询自身的失败时才是引擎的原话（解析失败、求值失败、被叫停的 governor）；这个端点无法归因于查询本身的缺陷——`resolveService`/`resolveLoad` 抛出异常，或适配器未能分类的任何其他异常——绝不会以自身的消息或调用栈进入响应：客户端得到的是固定的通用 `detail` 与一个 `correlationId`，真正的错误只会交给 `onInternalError`（默认输出一行 `console.error`）。超出上限的 `Content-Length` 会在任何内容被读取之前就被拒绝；缺失或偏小的 `Content-Length` 同样会被捕获——请求体流入时按字节计数，谎报的请求头绝不会因此换来比诚实请求头更大的请求体。两个处理函数都以 `redirect: "manual"` 发起请求：`SERVICE` 请求绝不跟随重定向（3xx 是一个带类型的传输失败，因此被编目端点的请求头与凭据绝不会到达另一个源），而 `LOAD` 仅在对被重定向的 IRI 重新按目录鉴权之后才会跟随每一跳，跳数上限为 `maxRedirects`（默认 5）。一个完整的 Worker：

```js worker-recipe
import wasm from "@blackcatinformatics/purrdf/purrdf_wasm_bg.wasm";
import { ready, Dataset, QueryEngine, ServiceCatalog } from "@blackcatinformatics/purrdf";
import { createFetchServiceResolver, handleSparqlRequest } from "@blackcatinformatics/purrdf/cloudflare";

await ready(wasm);
const engine = new QueryEngine();
const dataset = Dataset.parse(
  "<https://example.org/a> <https://example.org/p> <https://example.org/o1> .\n",
  "nquads",
);
const catalog = new ServiceCatalog();
catalog.addService(
  "https://remote.example.org/sparql",
  JSON.stringify({ capabilities: ["query", "network"] }),
);

export default {
  fetch(request, env, ctx) {
    const resolveService = createFetchServiceResolver({
      catalog,
      timeoutMs: 5_000,
      bindings: { "https://remote.example.org": env.REMOTE },
      cache: caches.default,
      cacheTtlSeconds: 300,
      waitUntil: (promise) => ctx.waitUntil(promise),
    });
    return handleSparqlRequest(request, {
      engine,
      dataset,
      catalog,
      resolveService,
      cors: { origins: "*" },
      governors: { deadlineMs: 10_000, maxRemoteRequests: 40 },
    });
  },
};
```

Workers 限制单次调用可以发出的子请求数量，而 `maxRemoteRequests` 正是与之精确对应的控制项：每个 `SERVICE` 请求和每个 `LOAD` 都在到达处理函数之前计数，因此一个请求发出的子请求绝不会超过该上限。Cache API 在 `workers.dev` 主机名上不起任何作用，因此缓存在那里实际上处于关闭状态；在自定义域名上它可以正常工作。未配置缓存就启动的运行时（例如在本地运行的 workerd）会以"No Cache was configured"拒绝 `cache.match` 与 `cache.put`，处理函数把它当作优化层的失败，绝不当作查询的答案：被拒绝的 `match` 视为未命中，被拒绝的 `put` 绝不会丢弃远端已经返回的答案。`onCacheError(error, { operation, endpoint })` 会报告每一次这类失败（默认输出一行 `console.warn`），因此失败始终可见，绝不会被悄悄吞掉，查询也照常作答。在 Workers 上，`Date.now()` 在 CPU 密集执行期间不会前进，因此同步的 `deadlineMs` 在那里无法在 CPU 密集的工作中触发；异步通道则在每一次让出和每一次宿主请求时检查截止时间。

包的 [README](https://github.com/Blackcat-Informatics/purrdf/tree/main/crates/rdf-wasm/js#asynchronous-queries-federation-and-the-cloudflare-adapter) 是这些约定的完整参考。

## 范围与当前限制

- **仅限内存。** SPARQL 查询在内存数据集上运行。同步方法不安装任何 `SERVICE` 或 `LOAD` 来源，因此在那里，远程 `SERVICE` 或 `LOAD` 除非写作 `SILENT`，否则会显式失败；异步孪生方法只能经由宿主传入的处理函数到达远程端点。
- **各格式的三元组项。**`serialize` 是写入器原生通道：宾语位置的引用三元组项与 RDF 1.2 陈述层在 Turtle、N-Triples、N-Quads 与 TriG
  （写作 `<<( … )>>`）、RDF/XML（写作 `rdf:parseType="Triple"`）以及 JSON-LD /
  YAML-LD（写作 `@triple`）中都得以保留。TriX 与 HexTuples 不支持三元组项，因此把
  携带三元组项的数据集序列化到二者之一会**抛出异常**，而不是静默丢弃该层。单图目标
  （Turtle、N-Triples、RDF/XML）只发出默认图。

## 从源码构建

Rust cdylib 位于
[`crates/rdf-wasm`](https://github.com/Blackcat-Informatics/purrdf/tree/main/crates/rdf-wasm)；
发布的 ESM 包由它生成：

```sh
make wasm-pkg        # release wasm + wasm-bindgen ESM bindings → js/pkg/
make wasm-pkg-test   # the above + TypeScript, Node, and packed-tarball gates
```

这需要 `wasm32-unknown-unknown` Rust 目标，以及固定到本 crate 所用 `wasm-bindgen`
版本的 `wasm-bindgen-cli`。
