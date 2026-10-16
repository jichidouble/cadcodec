# DWG 输出能力与 ODA 验证报告

验证对象：`acadrust 0.4.1` 的 DWG 写入器（`DwgWriter`）。
验证工具：ODA File Converter 27.1.0（`ODADrawingsExplorer 27.1.0\OdaFileConverter.exe`）。

## 验证方法

1. 运行 `tests/dwg_output_matrix.rs`，用代表性文档（LINE、CIRCLE、ARC、LWPOLYLINE、SPLINE、TEXT、MTEXT、DIMENSION、INSERT、POINT、HATCH、VIEWPORT，共 12 个实体）写出全部 8 个支持版本：

   ```powershell
   cargo test --test dwg_output_matrix
   ```

2. 将产物交给 ODA 批量转换为 ACAD2018，任何 `.err` 文件即视为该版本失败：

   ```powershell
   $env:ODA_FILE_CONVERTER = "D:\Program Files\ODA\ODADrawingsExplorer 27.1.0\OdaFileConverter.exe"
   cargo test --test dwg_output_matrix -- --ignored
   ```

   产物目录：`target/dwg_output_matrix/`，ODA 输出：`target/dwg_output_matrix/oda_out_2018/`。

## 结果

| DWG 版本 | 写入 | 库内读回 | ODA 验证 |
|---|---|---|---|
| R13 (AC1012) | 通过 | 通过 | 拒绝：`Object improperly read: <AcDbMLeaderStyle>` |
| R14 (AC1014) | 通过 | 通过 | 拒绝：`Object improperly read: <AcDbMLeaderStyle>` |
| R2000 (AC1015) | 通过 | 通过 | 通过 |
| R2004 (AC1018) | 通过 | 通过 | 通过 |
| R2007 (AC1021) | 通过 | 通过 | 通过 |
| R2010 (AC1024) | 通过 | 通过 | 通过 |
| R2013 (AC1027) | 通过 | 通过 | 通过 |
| R2018 (AC1032) | 通过 | 通过 | 通过 |

## 已知问题：R13/R14 输出包含 R2007+ 对象

`CadDocument::with_version`（`initialize_defaults`）无条件创建 `MultiLeaderStyle` 与
`TableStyle` 对象（`src/document.rs`），而 `DwgObjectWriter::write_object`
（`src/io/dwg/dwg_stream_writers/object_writer/objects.rs`）不做目标版本过滤。

因此 R13/R14 文件里携带了这些版本中不存在的对象，ODA 在解析时报
`Object improperly read: <AcDbMLeaderStyle>`；库自身读回不受影响（`objects`
和字典仍成环）。LibreDWG 对此宽容，可以解析。

修复方向：写入时按目标版本过滤对象（R2007 之前跳过 MLEADERSTYLE/TABLESTYLE），
并同步清理根字典条目与头部引用（`current_mleader_style_name` 等），避免悬挂引用。

## 已修复：R2010+ VisualStyle 序列化（2026-08-14）

根因：`VisualStyle::core_properties` 的 legacy(24)→R2010+ core(28) 映射表错位。
`legacy[19]`（display_settings）被映射到 `modern[12]`（本应是
edge_intersection_ltype），`legacy[6]` 被推到 `modern[13]`，且 `legacy[18]`
（edge_do_hide_precision）未映射，导致 `modern[12]` 起所有字段错位、位流错乱。

修复：`stub_objects.rs` 新增 `VisualStyle::legacy_to_core`（正确的 24→28 顺序映射），
`core_properties` 与 `legacy_properties` 均改用该映射；新增单元测试锁定字段位置。
修复后 cn.dxf（含 24 个 VisualStyle）的 R2010/R2013/R2018 输出均通过 ODA 验证。

> 上游同步后说明（合并 `hakanaktt/acadrust` main `a1a216d`）：VisualStyle 的 reader/writer
> 已整体采用上游重写版本，`core_properties` 的 legacy(24)→core(28) 映射表也随之改为上游表
> （`legacy[19]`→`modern[12]`）；本节所述的 fork 侧 `VisualStyle::legacy_to_core`（旧表）
> 已被取代并随后删除，单元测试 `core_properties_follows_r2010_legacy_mapping` 现锁定上游表。
> 上面的 ODA 验证结论只适用于当时的分支状态，若要在新基线上继续沿用，需要重新验证。

## VisualStyle 全版本专项验证（tests/dwg_visualstyle_matrix.rs）

用含 24 个 VisualStyle 的 cn.dxf 输出全部 8 个版本，ODA 验证结果：

| 版本 | 写入 | 库内读回 | ODA 解析 |
|---|---|---|---|
| R13 / R14 | 通过 | 通过（VisualStyle 完好） | 拒绝（原因：`AcDbMLeaderStyle`，与 VisualStyle 无关） |
| R2000 / R2004 / R2007 / R2010 / R2013 / R2018 | 通过 | 通过 | 全部通过 |

结论：VisualStyle 对象（R2007 legacy 格式读入）在全部支持版本中都能正确写入且库内读回
不丢失；ODA 可解析 R2000+ 的全部输出。R13/R14 的拒绝是已知的 MLeaderStyle 版本过滤
缺陷，修复后应可补齐最后两个版本。

## 已修复：R13/R14 MLeaderStyle 版本过滤（2026-08-14）

根因：`is_writable_object` 只对 VisualStyle/Material 按版本过滤，
`MultiLeaderStyle` 无条件可写、`TableStyle` 仅检查 class 存在；
`write_dictionary` 的 pre-R2000 根字典条目过滤列表漏掉了
`ACAD_MLEADERSTYLE` / `ACAD_TABLESTYLE`，导致 R13/R14 输出携带 R2007+ 对象。

修复：两处各一行级别改动（`objects.rs`）——`MultiLeaderStyle`/`TableStyle`
按 `r2007_plus()` 过滤（保留 R13/R14 同版本 raw passthrough 例外），
pre-R2000 根字典过滤加入 `ACAD_MLEADERSTYLE`/`ACAD_TABLESTYLE`。
验证：cn.dxf 全版本矩阵 **R13–R2018 共 8/8 全部通过 ODA**。

## 复现命令

```powershell
# 全版本生成 + 库内读回
cargo test --test dwg_output_matrix

# ODA 批量验证（需要本机安装 ODA File Converter）
$env:ODA_FILE_CONVERTER = "D:\Program Files\ODA\ODADrawingsExplorer 27.1.0\OdaFileConverter.exe"
cargo test --test dwg_output_matrix -- --ignored
```
