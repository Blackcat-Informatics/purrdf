<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->


[English](README.md) | 简体中文
# purrdf-geo

PurRDF 的 GeoSPARQL 兼容入口。它通过既有 Rust 路径重新导出共享几何内核，并提供求值器的 `functions` 和 `relation` 模块。几何与数值计算位于 `purrdf-geo-kernel`；SPARQL 注册与执行位于 `purrdf-sparql-eval`。

## 标准函数与显式配置

原生 SPARQL 引擎安装标准 GeoSPARQL 标量函数注册。标准 WKT 与 GeoJSON 使用 WGS84 经度／纬度。`GeoProfile` 提供额外参考系、坐标轴顺序、输出单位和实际坐标操作链。由调用方命名的 `GeoVocab` 注册仍遵循其显式平面几何契约。缺少必需配置时返回类型化拒绝，绝不虚构坐标操作，也不返回假的几何关系结果。

## 精确坐标与明示的数值法则

十进制坐标按精确有理数读取。平面几何的方向、相交、环绕向及 DE-9IM 判定使用整数／有理数运算。平面测量实现按明示内部尺度计算整数平方根。对这一缩放法则，整数计算是精确的；这并不意味着无理的物理长度成为精确值。每条线段的截断值不大于真实长度，误差严格小于一个尺度单位；总误差界随线段数量增长。输出舍入规则是显式的。

内核还提供带证明的测地计算、椭球度量、坐标操作、物理缓冲区、层次单元／覆盖以及空间索引。这些操作保留其声明的度量／参考系法则与预算。参数错误遵循 SPARQL 表达式错误语义。操作层面的精度、收敛、工作量、内存、环境和参考系配置拒绝仍是致命错误，不能变成 FILTER 的假值或 BIND 的未绑定值。

## 可移植执行

`make geo-determinism` 在原生、可移植 wasm 与 SIMD wasm 上执行已注册的平面几何、地理计算、操作、缓冲区、覆盖和索引控制。仅编译成功不能证明完成后的答案一致。参见[内核 README](../geo-kernel/README.md)与[精确性设计](../../docs/design/purrdf-geo-exactness.md)。

## 许可证

可自行选择以下任一许可证：

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
