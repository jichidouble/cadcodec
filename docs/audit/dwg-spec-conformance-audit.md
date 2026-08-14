# DWG 规范一致性审计（对照 DWG_File_Format_Specification_CN_5.4.2.md）

审计基准：`docs/DWG_File_Format_Specification_CN_5.4.2.md`（ODA 5.4.2 / LibreDWG 修订版，R13–R2018），
辅以本地 LibreDWG 参考实现（`D:\Projects\libredwg`）与 ODA File Converter 交叉验证。
范围：DWG 读写器（`src/io/dwg/**`）与对象/实体模型（`src/objects/**`、`src/entities/**`）。

## 已验证符合规范（本轮核对，含证据）

| 规范条款 | 实现 | 证据 |
|---|---|---|
| §4.3/4.4 BS/BL 分支（负值用 `00`、255 用 `01`、256 用 `00`） | ✅ | `bit_writer.rs` `write_bit_short/write_bit_long`；新增解码式测试锁定分支 |
| §4.5 BLL 3-bit 长度上限（LibreDWG “max 7 byte by design”） | ✅ | `write_bit_long_long` 对 8 字节值断言 `caps at 7`；测试覆盖 1–7 字节边界 |
| §4.7 MC 无符号（Object Map handle 增量禁负） | ✅ | `write_modular_char` / `read_modular_char`；`0x40` 解码为 +64 测试 |
| §4.8 MS 15-bit 模块 + 续传位 | ✅ | `write_modular_short(0x8000)` → `[00 80 01 00]`（低模块 0 + 高模块 1） |
| §10.3 R2018 MLINESTYLE 用 handle 引用 linetype | ✅ | writer `objects.rs:1864`（`r2018_plus` 分支写 HardPointer）；reader `objects.rs:1109` 对称 |
| §10.3 R2018 AuxHeader 末尾 3 个零 Int16 | ✅ | `aux_header_writer.rs` `version >= AC1032` 分支 |
| §10.2/21 R2013+ AcDsPrototype_1b（ACIS SAB 存储） | ✅ | reader `dwg_reader.rs` SAB 提取；writer `dwg_writer.rs` `build_acds_prototype`；roundtrip 测试覆盖 |
| §18.4 TABLESTYLE annotative（AcadAnnotative EED，非原生字段） | ✅（本次修复） | writer 写入 EED；builder 读取 EED；`dwg_roundtrip_annotative_styles` 含 table 断言 |
| §18.4 MLEADERSTYLE annotative（原生 DWG 位 + DXF 296） | ✅ | writer/reader 均已实现；DXF/DWG 往返测试 |
| §18.4 VISUALSTYLE/MATERIAL/MLEADERSTYLE/TABLESTYLE 版本过滤（pre-R2007 不写） | ✅ | `is_writable_object` + 根字典条目过滤；ODA 8/8 版本矩阵通过 |
| §18.3 XRECORD 未知组码按值类型保留 | ✅ | reader `_ =>` 按 code 类别映射；writer 原样回写 |
| §18.4 GEODATA | ✅ | `write_geodata`/`read_geodata` + 单元往返测试 |
| §18.3 DICTIONARY / DICTIONARYWDFLT（cloning/ownership flags、default handle） | ✅ | writer/reader 对称实现 |
| §18.4 未知类对象原样保真（raw passthrough，仅同版本族） | ✅ | `ObjectType::Unknown` + `raw_passthrough_compatible` 跨版本丢弃 |

## 本轮 TDD 驱动的修复

1. **TABLESTYLE DWG annotative 丢失**（测试先红后绿）：
   - 写入端 `object_writer/objects.rs`：改用 `write_common_non_entity_data_eed` 携带 `AcadAnnotative` EED；
   - 读取端 `dwg_document_builder.rs`：解析 EED 并回填 `TableStyle::annotative`；
   - 测试：`tests/roundtrip.rs` 增加 DWG table 断言；新增 `tests/annotative_roundtrip_tdd.rs`（单对象/双对象隔离 + 失败时对象清单）。
2. **BLL 8 字节值静默损坏**：`write_bit_long_long` 对 `size > 7` 断言（符合 §4.5 3-bit 长度上限），
   新增 `#[should_panic]` 测试与 1–7 字节边界往返测试。
3. **位编码规范测试**：BS/BL 分支、MC 无符号、MS 边界字（§4.3–4.8）用位流解码断言，替代错误的字节索引断言。
4. **LAYER 真彩色回归测试**：`tests/dxf_layer_true_color.rs`（组码 420 在负 ACI 索引时仍保留 RGB）。

## 已知缺口 / 待确认（按优先级）

1. **MLeaderAnnotContext 无类型化支持**（规范 18.4 上下文/注释比例类）：全库零命中；
   当前依赖未知对象 raw 原样往返（同版本保真、跨版本丢弃）。要类型化需按 20.4.86–87 prescription 实现。
2. **`is_writable_object` 仅过滤 4 类 R2007+ 对象**：ObjectContextData、GeoData、SpatialFilter、
   Field/FieldList、DynamicBlock/Associative、DgnLineStyle 等走 `_ => true` 无条件写入；
   降版本（R2007→R2000 等）时可能向旧格式写入新对象。建议按各类型引入版本阈值并用 ODA 矩阵验证。
3. **DICTIONARY 重复 key 诊断**（规范 18.3 “重复 key 应诊断”）：读取端未见显式诊断/通知。
4. **R13/R14 解析兼容性**：LibreDWG 产出的 AC15 文件对象数读出偏少（既有问题，未在本轮处理）。
5. **annotative 测试并行稳定性**：早期观察到全量并行运行时偶发失败；修复 TABLESTYLE EED 后
   roundtrip 套件 5/5 稳定，但根因未做最终隔离（怀疑与测试中途编辑状态有关）。
6. **R2013 maintenance version**：代码写 29（0x1D），规范 10.2 提到 6/8；ODA 已接受产物，
   但与真实 R2013 文件字段值的对应关系建议再核对。
