# DWG 文件格式规格书（R13–R2018）— ODA 5.4.2-libredwg 修订版

<a id="dwg-document-status"></a>
## 1. 文档范围、来源与解释规则

本文是一份面向工程实现和 AI 检索的 DWG 二进制格式中文规格书，覆盖 AC1012（R13）至 AC1032（R2018），并附带 pre-R13 版本参考。主要依据 Open Design Alliance（ODA）《Open Design Specification for .dwg files》Version 5.4.1，并结合该 PDF 的完整文字层重新组织；本修订版进一步并入 5.4.2–libredwg 的变化（见 `doc/ODA-5.4.2-libredwg.patch`）：版本命名统一、R2004 页头 checksum 字段顺序更正、Second File Header 重写、codepage 全表、以及 ACSH/SUN 等新对象 prescription。本文不是 Autodesk 官方规范；DWG 是专有格式，ODA 原文也包含由逆向工程得到的未确认内容。

本文使用以下状态标签：

| 标签 | 含义 |
|---|---|
| **确定** | 原规范给出明确的字段、算法或常量 |
| **版本条件** | 仅适用于指定版本或维护版本 |
| **ODA 推测** | 原规范使用疑问语气或未确认含义 |
| **未知** | 已知长度或位置，但语义未知 |
| **实现建议** | 为构造健壮读写器而给出的派生建议，不属于磁盘格式 |

页码均指 Version 5.4.1 PDF 的印刷页码。字段顺序默认就是磁盘读取顺序；除特别注明外，整数和浮点数均为小端序、最低有效字节在前。位编码必须从当前位位置连续读取，不能擅自对齐到字节边界。

<a id="dwg-version-matrix"></a>
## 2. 版本标识与格式谱系

DWG 文件前 6 字节是 ASCII 版本标识：

| 文件标识 | AutoCAD 代际 | 本文简称 | 主要结构 |
|---|---:|---|---|
| `MC0.0` | MicroCAD R1.1 | pre-R13 | 最早格式 |
| `AC1.2`–`AC1.50` | R1.2–R2.0 | pre-R13 | 旧式线性格式 |
| `AC2.10`–`AC2.22` | R2.10–R2.22 | pre-R13 | 旧式线性格式 |
| `AC1001`–`AC1009` | R2.4–R11/R12 | pre-R13 | 旧式 section 结构 |
| `AC1012` | R13 | R13 | 旧式 section locator、sentinel、绝对对象位置 |
| `AC1013` | R13C3 | R13C3 | R13 维护版，结构变化点 |
| `AC1014` | R14 | R14 | 与 R13 同族，字段有所扩展 |
| `AC1015` | 2000–2002 | R2000/R15 | R13–R15 容器末代，新增 AuxHeader 等 |
| `AC1016` | 2000i | R2000i | 与 R2000 同结构 |
| `AC1018` | 2004–2006 | R2004/R18 | page map、section map、逻辑数据区、LZ77 压缩 |
| `AC1021` | 2007–2009 | R2007/R21 | 新文件头、CRC-64、Reed–Solomon、独立字符串流 |
| `AC1024` | 2010–2012 | R2010/R24 | R2004 容器组织 + R2007 对象流编码，object type 改码 |
| `AC1027` | 2013–2017 | R2013/R27 | 新增 `AcDb:AcDsPrototype_1b` DataStorage |
| `AC1032` | 2018+ | R2018/R32 | 结构与 R2013 相同，部分实体和 AuxHeader 扩展 |

必须把“容器组织”和“对象编码”分开判断：R2010/R2013 的文件头、page map、section map 和压缩主要沿用 R2004，而数据、字符串和句柄分流沿用 R2007。不能仅按版本号假定后一个版本完整继承前一个版本。（原规范 pp.20–71）

5.4.2 起全文统一使用 AutoCAD 发行名（R2000/R2004/R2007/R2010/R2013/R2018），不再使用 R15/R18/R21/R24/R27 等内部代号。

<a id="dwg-terms"></a>
## 3. 存储模型与术语

- **物理文件**：连续字节流，包含固定头、系统页、数据页和尾部结构。
- **section / data section**：命名逻辑数据区，例如 `AcDb:Header`、`AcDb:Classes`、`AcDb:AcDbObjects`。
- **page**：一个 section 被分割、压缩、校验或纠错后的物理存储单元。
- **object**：DWG 中所有可由 handle 标识的记录总称，包括 entity、table record、control object、dictionary object 等。
- **entity**：可显示或参与图形数据库的对象，具有公共实体字段。
- **handle**：对象标识；handle reference 同时编码引用关系或相对偏移。
- **seeker**：`RL` 类型的绝对文件地址或相对已知地址的偏移。
- **sentinel (`SN`)**：16 字节恢复标记，用于识别 section 边界。
- **data stream**：对象主体位流。
- **string stream**：R2007+ 对象中的 Unicode 字符串位流。
- **handle stream**：集中保存对象引用的位流。
- **EED**：Extended Entity/Object Data，磁盘中的扩展应用数据；概念上对应 DXF XData。

<a id="dwg-basic-types"></a>
## 4. 基本数据类型和位编码

### 4.1 缩写总表

| 缩写 | 含义 | 大小/说明 | 来源 |
|---|---|---|---:|
| `B` | bit | 1 bit | p.7 |
| `BB` | bit pair | 2 bits | p.7 |
| `3B` | bit triplet | 读取 1–3 bits，R2010+ | pp.7–8 |
| `RC` | raw char | 1 byte，不压缩 | p.7 |
| `RS` | raw short | 2 bytes，不压缩 | p.7 |
| `RL` | raw long | 4 bytes，不压缩 | p.7 |
| `RD` | raw double | 8-byte IEEE-754 | p.7 |
| `BS` | BitShort | 压缩 16-bit 值 | pp.8–9 |
| `BL` | BitLong | 压缩 32-bit 值 | p.9 |
| `BLL` | BitLongLong | 压缩 64-bit 值，R2010+ | p.9 |
| `BD` | BitDouble | 压缩 double | p.9 |
| `2BD`/`3BD` | 点/向量 | 2 或 3 个 `BD` | p.7 |
| `2RD`/`3RD` | 点/向量 | 2 或 3 个 `RD` | p.7 |
| `MC` | Modular Char | 7-bit 模块变长整数 | pp.10–12 |
| `MS` | Modular Short | 15-bit 模块变长整数 | pp.12–13 |
| `H` | Handle reference | code、长度、handle/offset | pp.15–16 |
| `T` | 旧式文本 | `BS` 长度 + 单字节字符串 | p.7 |
| `TU` | Unicode 文本 | `BS` 字符数 + UTF-16 code units | p.7 |
| `TV` | 版本文本 | R2004- 为 `T`，R2007+ 为 `TU` | p.7 |
| `SN` | sentinel | 16 bytes | p.7 |
| `BE` | BitExtrusion | 带默认值的 extrusion | p.13 |
| `DD` | BitDouble Default | 相对默认 double 的压缩值 | pp.13–14 |
| `BT` | BitThickness | 带零默认值的 thickness | p.14 |
| `CMC`/`ENC` | 颜色 | 通用颜色/实体颜色 | p.14 |
| `OT` | Object Type | 版本相关类型编码 | p.15 |
| `X` | 特殊结构 | 由上下文定义 | p.7 |
| `U` | 未知 | 语义未知 | p.7 |

<a id="dwg-bitcode-3b"></a>
### 4.2 `3B` — BitTriplet

逐 bit 读取，遇到 `0` 或读满第 3 bit 时停止。每读取一位，将既有结果左移后合入该位，结果范围为 0–7。仅用于 R2010+ 的若干紧凑枚举。（原规范 p.8）

<a id="dwg-bitcode-bs"></a>
### 4.3 `BS` — BitShort

| 前导 2 bits | 后续数据 | 结果 |
|---|---|---|
| `00` | 2-byte little-endian short | 有符号 16-bit 值 |
| `01` | 1-byte unsigned char | 0–255 |
| `10` | 无 | 0 |
| `11` | 无 | 256 |

负值必须使用 `00`。例如 `-1` 编码为 `00` 后跟 `FF FF`，不能用 char 分支。（pp.8–9）

<a id="dwg-bitcode-bl"></a>
### 4.4 `BL` — BitLong

| 前导 2 bits | 后续数据 | 结果 |
|---|---|---|
| `00` | 4-byte little-endian long | 有符号 32-bit 值 |
| `01` | 1-byte unsigned char | 0–255 |
| `10` | 无 | 0 |
| `11` | 未使用 | 不应由写入器生成 |

负值和大于等于 256 的值使用 `00`。（p.9）

<a id="dwg-bitcode-bll"></a>
### 4.5 `BLL` — BitLongLong

先读取 `3B` 得到字节数 `l`，再读取 `l` 个字节，最低有效字节在前。适用于 R2010+。（p.9）

<a id="dwg-bitcode-bd"></a>
### 4.6 `BD` — BitDouble

| 前导 2 bits | 后续数据 | 结果 |
|---|---|---|
| `00` | 8-byte `RD` | 指定 double |
| `01` | 无 | `1.0` |
| `10` | 无 | `0.0` |
| `11` | 未使用 | 不应由写入器生成 |

<a id="dwg-bitcode-mc"></a>
### 4.7 `MC` — Modular Char

`MC` 是以 7 bits 为一个数据模块的变长整数。每字节 bit 7 是 continuation flag：为 1 表示仍有下一字节，为 0 表示结束。第一个模块承载最低有效 7 bits。将各字节去掉最高 flag bit 后，以 7-bit 小端模块重组整数。

终止字节的 bit 6（`0x40`）通常表示负号；清除该位后重组绝对值，再取负。Object Map 的 handle 增量禁止负值，但文件位置增量允许负值。实现必须设置最大模块数，防止恶意输入形成无限或溢出的 varint。（pp.10–12）

<a id="dwg-bitcode-ms"></a>
### 4.8 `MS` — Modular Short

以 16-bit little-endian short 为模块，每个模块的最高 bit 是 continuation flag，其余 15 bits 是数值。低位模块先出现。主要用于对象记录字节长度。原规范给出的实际范围通常只需 1–2 个模块；读取器仍应检查溢出。（pp.12–13）

<a id="dwg-bitcode-defaults"></a>
### 4.9 默认值编码 `BE`、`BT`、`DD`

`BE`：R13–R14 直接读取 `3BD`。R2000+ 先读 `B`；为 1 时值默认为 `(0,0,1)`，为 0 时再读 `3BD`。

`BT`：R13–R14 直接读取 `BD`。R2000+ 先读 `B`；为 1 时值为 `0.0`，为 0 时再读 `BD`。

`DD` 需要调用方提供一个默认 `RD`：

| opcode | 数据 | 结果构造 |
|---|---|---|
| `00` | 无 | 使用默认值 |
| `01` | 4 bytes | 替换默认值小端表示的 bytes 0–3 |
| `10` | 6 bytes | 前 2 bytes 替换默认值 bytes 4–5，后 4 bytes 替换 bytes 0–3 |
| `11` | 完整 `RD` | 使用完整 double |

`3DD` 是三个分别带默认值的 `DD`。（pp.13–14）

<a id="dwg-color"></a>
### 4.10 颜色 `CMC` 与 `ENC`

R15 及以前颜色通常是 `BS` ACI 索引。R2004+ 的 `CMC` 由 `BS` color index（通常为 0）、`BL` RGB 值和 `RC` flags 组成；flags bit 0 表示随后有颜色名称 `TV`，bit 1 表示随后有 color-book 名称 `TV`。

实体使用 `ENC`：先读 `BS color number`，高位是特性标志，低位是 ACI：

- `0x8000`：complex/RGB color，后跟包含 24-bit RGB 的值；
- `0x4000`：有 `AcDbColor` handle，该 handle 位于 handle stream；
- `0x2000`：后跟 transparency `BL`；其首字节类型 0=`BYLAYER`，1=`BYBLOCK`，3=末字节为透明度值；
- 无上述标志时按 ACI 解释。

（原规范 p.14；部分原文对 RGB 后续值类型的描述存在不一致，读取时应以版本样本和对象边界共同验证。）

<a id="dwg-object-type"></a>
### 4.11 `OT` — Object Type

R2007 及以前按 `BS` 读取。R2010+ 先读 `BB`：

| `BB` | 后续 | type |
|---|---|---|
| 0 | `RC` | byte value |
| 1 | `RC` | byte value + `0x1F0` |
| 2 | `RS` | raw short |
| 3 | `RS` | 保守地按分支 2 处理；规范称不应出现 |

<a id="dwg-text-encoding"></a>
### 4.12 文本和字符串流

R2004 及以前 `TV=T`，按图纸 code page 解释单字节字符。R2007+ `TV=TU`，每字符 2 bytes。对象描述中出现 `TU/TV` 只表示语义顺序；R2007+ 的实际字符串通常必须从对象的独立 string stream 读取，而不是从 data stream 的当前位置读取。字符串长度是 code-unit 数而不是字节数。（pp.7, 54, 104）

<a id="dwg-handle-reference"></a>
## 5. Handle、引用关系与对象图

### 5.1 磁盘形式

Handle reference 首字节由两个 nibble 组成：`CODE(4 bits) | COUNTER(4 bits)`，随后是 `COUNTER` 个 handle 或 offset 字节。文档简写如 `5.2.05.E7`。Handle 字节按大端数值顺序表达，即例中 `05 E7` 代表 `0x05E7`。多数情况下 COUNTER 不超过 4（handle 最大 32 bit）；个别文件出现 COUNTER=5，揭示 handle 中存在未公开 bit，但值仍按 32 bit 解释。（pp.15–16；5.4.2 补充）

### 5.2 关系代码

| code | 关系/计算 |
|---:|---|
| `0x2` | soft ownership，绝对 handle |
| `0x3` | hard ownership，绝对 handle |
| `0x4` | soft pointer，绝对 handle |
| `0x5` | hard pointer，绝对 handle |
| `0x6` | reference handle + 1，counter 为 0 |
| `0x8` | reference handle - 1，counter 为 0 |
| `0xA` | reference handle + offset |
| `0xC` | reference handle - offset |

原规范表格排版跨行；关系语义以 ownership/pointer 的通用定义为准。Ownership 决定对象生命周期，pointer 只表达引用；hard 表示引用方需要目标存在，soft 表示目标可独立缺失。

### 5.3 对象图规则

`AcDb:AcDbObjects` 中记录可以任意排序，也可能包含未使用对象和重名 table records。正确数据库不是“section 中所有物理记录”的集合，而是从 Header 根 handle、control objects 和 owner links 可达的对象图。Object Map 用于把 handle 定位到对象记录，但不自动证明对象在逻辑数据库中可达。（p.103）

<a id="dwg-integrity"></a>
## 6. 边界、CRC 与纠错

### 6.1 CRC-16

原规范称其为“8-bit CRC”算法，但输出为 2 bytes，本质是查 256×16-bit 表的 CRC-16 变体。CRC 总在字节边界出现；对象主体若未对齐，先跳过 padding bits。对象 CRC 覆盖对象记录的 size bytes。R13–R15 Object Map 每个 chunk 的 CRC 以高字节在前，是局部大端例外。（pp.16–18, 251）

### 6.2 CRC-32

R2004 文件组织使用标准样式的 32-bit CRC/校验字段，具体 seed 和覆盖范围由所在 header/page 定义，不能将所有 CRC-32 统一套用同一个范围。（p.18 及第 4 章）

### 6.3 R2007 CRC-64

R2007 使用 ECMA-182 多项式体系的 normal 和 mirrored 两种 CRC-64。两者查表和移位方向不同；normal 结果最终按位取反，mirrored 不取反。输入不是简单从左到右：按 1–8 字节余数表规定的块顺序重排，完整 8-byte 块顺序等价于 `[6,7,4,5,2,3,0,1]`。初始化 seed 还依上下文经过 `UpdateSeed1` 或 `UpdateSeed2`，不能用固定初值替代。（pp.63–68）

### 6.4 Reed–Solomon

R2007 的 system pages 使用 RS(255,239)，每 239 data bytes 加 16 parity bytes，可修正 8 个错误字节；data pages 使用 RS(255,251)，每 251 data bytes 加 4 parity bytes，可修正 2 个错误字节。编码可能 interleaved 或 non-interleaved；interleaved 时多个 codeword 按相同字节位置交织。末块不足 `k` bytes 时以 CRC random encoder 产生的随机字节补齐。（p.68）

### 6.5 实现边界要求

读取任何长度、计数、页偏移或 handle 前必须验证：加法/乘法不溢出、目标在逻辑 section 和物理文件范围内、解压后尺寸不超过声明上限、递归对象/EED/JSON 类结构有深度限制。CRC 不匹配应产生结构化诊断；是否继续由严格/容错模式决定。

<a id="dwg-r13-container"></a>
## 7. R13–R15 文件组织

### 7.1 总体顺序

R13C2 与 R13C3 之间有结构变化。一般顺序如下（方括号表示可选或版本条件）：

```text
File Header
AcDb:Header variables + CRC
AcDb:Classes
[R13 Template]
[R13C3+ Padding，约 200 bytes，存在 Template 时相应缩减]
[pre-R13C3 Preview/Image]
Object data
Object Map
[Object Free Space（R14–R2000，可选）]
Second File Header
[Template（R14–R2000，可选，含 MEASUREMENT）]
[R13C3+ Preview/Image]
```

（原规范 p.20；5.4.2 修订 ObjFreeSpace/Second Header/Template 顺序）

### 7.2 固定文件头

- `0x00..0x05`：ASCII version ID。
- `0x06` 起 7 bytes：通常六个零和一个 `1`；R14 中可包含 `ACADMAINTVER`。**ODA 推测：**其他观测值语义不明。
- `0x0D`：`RL`，preview/image beginning sentinel 的绝对地址。
- `0x13`：`RS`，DWG code page。
- `0x15`：`RL` section-locator record 数量。
- 每条 locator：`RC record number | RL absolute seeker | RL section size`。

已知 record（5.4.2 修订）：0=`Header variables`，1=`Classes`，2=`Object Map`，3=R13+ `Object Free Space`（无 sentinel，其后紧跟带 sentinel 的 Second File Header），4=R13–R2000 `Template`（含 MEASUREMENT 变量，可选），5=`AuxHeader`（可选）。最多见过 6 条；ODA Toolkit 只写前 5 条。locator 数据之后是 CRC-16（seed `0xC0C1`，覆盖从文件头到此处）和固定 header end sentinel。（pp.21–22；5.4.2 修订）

`0x13` 处 code page 取值表（5.4.2 补全）：

| 值 | 名称 | 值 | 名称 |
|---:|---|---:|---|
| 0 | UTF8（未用） | 23 | MACINTOSH |
| 1 | US_ASCII | 24 | BIG5 |
| 2 | ISO-8859-1 | 25 | CP949（韩文 Wansung+Johab） |
| 3 | ISO-8859-2 | 26 | JOHAB |
| 4 | ISO-8859-3 | 27 | CP866（俄文） |
| 5 | ISO-8859-4 | 28 | ANSI-1250（中欧/东欧） |
| 6 | ISO-8859-5 | 29 | ANSI-1251（西里尔） |
| 7 | ISO-8859-6 | 30 | ANSI-1252（西欧） |
| 8 | ISO-8859-7 | 31 | GB2312（中文 EUC-CN） |
| 9 | ISO-8859-8 | 32 | ANSI-1253（希腊） |
| 10 | ISO-8859-9 | 33 | ANSI-1254（土耳其） |
| 11 | CP437（DOS 英文） | 34 | ANSI-1255（希伯来） |
| 12 | CP850（DOS Latin-1） | 35 | ANSI-1256（阿拉伯） |
| 13 | CP852（DOS 中欧） | 36 | ANSI-1257（波罗的） |
| 14 | CP855（DOS 西里尔） | 37 | ANSI-874（泰文） |
| 15 | CP857（DOS 土耳其） | 38 | ANSI-932（日文，windows-31j） |
| 16 | CP860（DOS 葡萄牙） | 39 | ANSI-936（简体中文） |
| 17 | CP861（DOS 冰岛） | 40 | ANSI-949（韩文 Wansung） |
| 18 | CP863（DOS 希伯来） | 41 | ANSI-950（繁体中文） |
| 19 | CP864（DOS 阿拉伯） | 42 | ANSI-1361（韩文 Wansung） |
| 20 | CP865（DOS 北欧） | 43 | UTF16（R2007 起默认） |
| 21 | CP869（DOS 希腊） | 44 | ANSI-1258（越南文） |
| 22 | CP932（DOS 日文 shiftjis） | | |

### 7.3 sentinel 和 section

Header、Classes 等旧式 section 通常由 16-byte beginning sentinel、长度/数据、CRC 和对应 end sentinel 包围。Sentinel 是恢复和同步辅助，不应代替 locator 中的受边界检查偏移。读取器应同时验证 locator、sentinel 和 CRC；容错恢复时才扫描 sentinel。

### 7.4 Second File Header

R13–R2000 尾部、紧随 ObjFreeSpace section 之后有冗余 second header，begin sentinel 为：

```text
D4 7B 21 CE 28 93 9F BF 53 24 40 09 12 3C AA 01
```

5.4.2 重写后的布局（旧版"14 组 control object handles"描述废弃）：

```text
RL  section size
BL  location（本 header 起始 sentinel 的地址）
RC  "AC1012" / "AC1013" / "AC1014" / "AC1015"
RC  5 个 0
RC  maintenance release version
RC  0x00 / 0x01 / 0x03
BS  写入文件的应用版本（首字节 application version，次字节 maintenance release version）
RS  codepage
BS  section 数量
repeat section 数量:
    RC section id
    BL section address
    BL section size
end repeat
```

随后有 CRC-16（seed `0xC0C1`，覆盖 sentinel 之后数据）和 end sentinel。它用于恢复，不应在主结构有效时覆盖主 header。（pp.264–266；5.4.2 重写）

<a id="dwg-r2004-container"></a>
## 8. R2004 文件组织

### 8.1 核心模型

R2004 将命名 data sections 分割成 section pages。Section Map 描述逻辑 section 及其 pages；Section Page Map 描述物理页/空隙的顺序和大小。把某 section 的 pages 按 logical offset 拼接，可恢复一个连续逻辑 section。全零页可以不写入物理文件，因此 page count 不能直接当作逻辑页数。（pp.23–31）

### 8.2 文件头

文件头占前 `0x100` bytes。版本签名之后包含 maintenance version、preview seeker、应用版本、code page、security flags 以及用于定位 map 的字段。R2004 header 的主体经过固定 XOR/伪随机掩码处理；读取时必须先按规范解码，再解释字段和验证 CRC-32。

关键导航字段包括：

- section page map ID 与地址（规范指出地址值加 `0x100` 得实际 stream position）；
- section map ID；
- section page array size、gap array size；
- last section page ID、last section page end address；
- second header address；
- CRC-32。

### 8.3 Section Page Map

Page Map 是系统 section。解压后依次给出 page size 和 page number/ID；正 ID 表示物理 page，负 ID 表示 gap。第一个 section 从 `0x100` 开始，后续地址由前项 size 累加。System pages 位于 page 序列尾部，并使用特定 ID/magic 区分 page map 与 section map。（pp.26–28）

### 8.4 Section Map

Section Map 为每个命名 section 保存：size、page count、max page size、compressed/encrypted 属性、section name，以及每个 page 的 ID、logical offset、size 等。页面可乱序出现，必须按 logical offset 重组。注意 PageCount 只统计实际写入物理文件的页数：全零页可省略不写，若已读页总解压尺寸小于 section size，应补全零页直至相等（可通过页 start offset 大于按前序页累计值来检测缺失零页）。Section ID 编号规则：空名称 section 恒为 0，其余从 `(section 数 - 1)` 递减到 1。常见 section 见第 12 章。（pp.28–31；5.4.2 修订）

### 8.5 page header、checksum 与加密

System page header 与 data page header 不同。R2004 data page header（0x20 bytes）字段顺序（5.4.2 修订，5.4.1 中 0x14/0x18/0x1C 顺序有误）：

| 偏移 | 长度 | 含义 |
|---:|---:|---|
| 0x00 | 4 | section page type（数据段恒为 `0x4163043b`） |
| 0x04 | 4 | section number |
| 0x08 | 4 | data size（压缩后） |
| 0x0C | 4 | page size（解压后） |
| 0x10 | 4 | start offset（在解压 buffer 内） |
| 0x14 | 4 | **Unknown**（ODA 写 0） |
| 0x18 | 4 | **Page header checksum**（以 data checksum 为 seed，对未编码 header 字节计算） |
| 0x1C | 4 | **Data checksum**（以 0 为 seed，对压缩数据字节计算） |

每个 data page 必须在原始流中 0x20 对齐；页间空隙用 magic number sequence 填充。Checksum 以 header 字段和 page data 为输入并使用上下文 seed。加密 data-page header 会用与 page address/ID 相关的 XOR 序列变换；规范并未完整描述通用加密内容，**实现建议：**遇到 encrypted section 而无可靠密钥/算法时保留原始页并报告“不支持”，不要猜测解密。

<a id="dwg-r2004-lz77"></a>
### 8.6 R2004 LZ77 变体

压缩流交替描述 literal bytes 和从已解压输出向后复制的 match。Opcode 同时编码 literal length、match length 和 backward offset，较大值通过附加字节扩展。复制源允许与目标重叠，因此必须按 LZ77 逐字节/安全 overlap 语义复制。解压必须恰好产生声明的 uncompressed size；过早结束、越界 offset、输出超限均为格式错误。（pp.32–35）

<a id="dwg-r2007-container"></a>
## 9. R2007 文件组织

### 9.1 总体差异

R2007 仍由 sections/pages 构成，但文件头、系统页、page/section maps、压缩、CRC 和纠错机制均与 R2004 显著不同。文件前部包含版本区与随机/校验数据；核心 file header page 位于 `0x80`，长度 `0x400`，文件末尾还写一份副本。Page Map 和 Section Map 各有冗余副本。（pp.36–49）

### 9.2 Metadata 和导航

解码后的 metadata 含：

- `PagesMapOffset/Id`、压缩与未压缩大小和 CRC；
- `SectionsMapId`、压缩与未压缩大小和 CRC；
- correction factor、CRC seeds、最大 page ID；
- file size、stream version、maintenance version、security flags；
- header CRC、encoded seed、random/check values。

Page Map 解码为重复的 `Int64 size, Int64 id`。以累计 `offset` 计算每个 page 的物理位置；使用 `abs(id)` 作为索引，负 ID 保留 gap/特殊语义。Section Map 从 `SectionsMapId` 对应 page 的 offset 加载。（pp.37–41）

### 9.3 R2007 Section Map

每个 section 条目含 Unicode name、64-bit size、max page size、encryption/compression/encoding 属性和 page 数；每页含 data offset、page size、ID、uncompressed/compressed size、checksum 与 CRC。若 page data offset 小于已重建数据长度，表示覆盖或零页式逻辑布局，必须按条目规则处理而非简单 append。（pp.41–45）

### 9.4 File Header page 写入/读取流水线

规范顺序不可交换：

1. 初始化 `RandomSeed`、`CrcSeed`、map CRC seeds；
2. 构造 metadata，计算 decompressed header 的 normal CRC-64；
3. 压缩；若压缩不变短则保存原数据，并按规定表示 size；
4. 计算 compressed data CRC 和 checking sequence；
5. 构造 32-byte blocks，加入 8-byte 对齐及 random padding；
6. 用 system-page RS(255,239) 交织编码，编码区为 `3 × 255` bytes；
7. 补齐 `0x400`，在 `0x3D8` 写入 `0x20` bytes check data；
8. 写到 `0x80` 和文件末尾。

读取顺序反向执行，并在每层分别验证 RS、compressed CRC、uncompressed CRC 和 check data。（pp.46–49）

<a id="dwg-r2007-system-page"></a>
### 9.5 System Section Page

仅 Page Map 和 Section Map 使用 system page。数据先算 mirrored CRC-64，再压缩（仅当更短），对压缩结果再算 CRC，补到 8-byte 边界，在页面中重复尽可能多次，然后使用 RS(255,239) interleaved 编码，余部用 random encoder 填充。页面至少 `0x400` bytes，并按 `0x20` 对齐；应至少容纳两份 aligned data。（pp.49–51）

### 9.6 Data Section Page

普通 section 使用 data page：

1. 按 max page size 切分逻辑 section；
2. 计算 32-bit data checksum；
3. 可选压缩，只有更短才采用；
4. 若 section encrypted，执行相应加密；
5. 计算 mirrored CRC-64；
6. 零填充至 8-byte；
7. 按 section encoding 使用 RS(255,251)，可能 interleaved（4）或 non-interleaved（1）；
8. page start/end 对齐到 `0x20`。

（pp.51–53）

<a id="dwg-r2007-lz77"></a>
### 9.7 R2007 LZ77 变体

R2007 使用不同于 R2004 的 opcode 方案。源流由 literal chunks 与从已解压缓冲区复制的 chunks 组成；literal copy 对 1–8 字节块有特殊重排顺序，match 的 length/offset 由 opcode nibble 和扩展字节共同组成。压缩和解压不能复用 R2004 实现。读取器必须验证 source 消耗、target 长度、offset 不为零且不超过已输出长度。（pp.55–59）

### 9.8 CRC random encoding

File header、system-page padding 和部分 check data 使用确定性的伪随机编码器。其状态由 seed 演进；调用顺序影响后续所有值。因此写入器必须严格保持规范步骤，读取器也必须用相同状态推进后再验证字段。（pp.59–63）

<a id="dwg-r2010-plus"></a>
## 10. R2010、R2013 与 R2018

### 10.1 R2010

R2010 的 file header、page map、section map 和 compression 与 R2004 相同；bit coding 与 R2007 相同，但 object type 使用第 4.11 节的新编码。Header 和 objects 中 data、strings、handles 分离。（p.69）

### 10.2 R2013

R2013 基于 R2010：容器和压缩仍按 R2004，位编码按 R2010，Header 仅增加少数字段。新增 `AcDb:AcDsPrototype_1b` DataStorage，用于 REGION、3DSOLID、BODY 等 ACIS/ASM 二进制数据。已知 maintenance version 包括 6 和 8，ODA 写 8；读取器不应只接受单一值。（p.70）

### 10.3 R2018

R2018 与 R2013 结构相同：AuxHeader 末尾增加三个零值 `Int16`（原文摘要和详细字段表对数量/类型排版有歧义，应以实际 AuxHeader 边界校验）；Header 开头的 maintenance-version 条件 unknown 32-bit 值仍存在；ACAD_PROXY_ENTITY、ATTRIB、ATTDEF、MTEXT 有字段变化；MLINESTYLE 元素用 handle 引用 linetype 而非索引。（p.71）

<a id="dwg-section-directory"></a>
## 11. 标准 Data Sections 总表

Section 属性中的 compressed/encrypted 是格式枚举，不应简单当成布尔值。

| Section | 主要内容 | 典型压缩 | 典型 page size | 原规范 |
|---|---|---:|---:|---:|
| `AcDb:Header` | 系统变量与根 handles | 2/版本相关 | `0x7400` 类 | pp.72–85 |
| `AcDb:Classes` | 动态 class 注册表 | 2/版本相关 | 版本相关 | pp.86–88 |
| `""` | **未知**数据区 | 2 | `0x7400` | p.90 |
| `AcDb:SummaryInfo` | 标题、作者等摘要 | 1 | `0x100` | pp.91–92 |
| `AcDb:Preview` | thumbnail/preview | 1 | 内容相关 | pp.93–94 |
| `AcDb:VBAProject` | VBA project bytes | 1 | 内容相关 | p.95 |
| `AcDb:AppInfo` | 应用/版本信息 | 1 | `0x80` | pp.96–97 |
| `AcDb:FileDepList` | 外部文件依赖 | 1 | `0x80 × entries` 类 | pp.98–99 |
| `AcDb:RevHistory` | 保存/修订历史 | 2 | `0x7400` | p.100 |
| `AcDb:Security` | 安全元数据 | 1 | `0x7400` | pp.101–102 |
| `AcDb:AcDbObjects` | 所有对象记录 | 2 | `0x7400` | pp.103–248 |
| `AcDb:ObjFreeSpace` | 对象空闲空间统计 | 2/版本相关 | 版本相关 | p.249 |
| `AcDb:Template` | measurement/模板设置 | 版本相关 | 小型 | p.250 |
| `AcDb:Handles` | Object Map | 2 | 版本相关 | p.251 |
| `AcDb:AcDsPrototype_1b` | R2013+ binary DataStorage | 2/版本相关 | 内容相关 | pp.252–262 |
| `AcDb:AuxHeader` | 冗余版本、保存次数、时间 | 版本相关 | 小型 | pp.267–268 |

<a id="dwg-header-section"></a>
## 12. `AcDb:Header` — Header Variables

### 12.1 包装结构

R13–R15 由 beginning sentinel、`RL section size`、bit-coded variables、handle references、CRC 和 end sentinel 组成。R2007+ 把字符串和 handles 分离到各自 stream；变量语义顺序不等于物理读取流。R2010/R2013 且 maintenance version > 3 时，section size 后存在一个 unknown 32-bit 值；R2018 也保留。（pp.72, 55, 71）

### 12.2 变量类别

原规范 pp.72–85 给出完整顺序，解析器必须逐版本执行字段 prescription。为便于检索，字段可分为：

| 类别 | 代表变量 | 存储特点 |
|---|---|---|
| 版本/维护 | `ACADVER`、`ACADMAINTVER`、code page | 常位于容器头与 Header 双处 |
| 几何范围 | `EXTMIN/EXTMAX`、`LIMMIN/LIMMAX`、`INSBASE` | `3BD`/`2RD`/版本条件 |
| 时间 | `TDCREATE`、`TDUPDATE`、`TDINDWG`、`TDUSRTIMER` | Julian day + milliseconds/day |
| 当前状态 | `ORTHOMODE`、`REGENMODE`、`FILLMODE`、`QTEXTMODE`、`LTSCALE` | bit/short/double |
| 尺寸标注 | `DIM*` 大量变量 | 多版本增删，含颜色、文本和 handles |
| 视图/空间 | `TILEMODE`、`UCS*`、`PUCS*`、`VIEWCTR` | model/paper 分支 |
| 显示/打印 | `CECOLOR`、`CELTYPE`、`CELWEIGHT`、`PSTYLEMODE` | 值与 handle 组合 |
| 安全/维护 | `FINGERPRINTGUID`、`VERSIONGUID`、`DWGPROPS` | 多为 `TV/TU` 或 flags |
| 根对象引用 | block/layer/style/ltype/view/ucs/vport/appid/dimstyle control handles、dictionary roots | 位于 handle stream |

### 12.3 根 handles

Header 末部保存数据库入口，例如 BLOCK、LAYER、STYLE、LTYPE、VIEW、UCS、VPORT、APPID、DIMSTYLE control objects，以及 named-object dictionary、layouts、plot styles、materials、visual styles 等版本相关 dictionary handles。读取器应先解析这些根引用，再构造对象可达图。HANDSEED 表示下一可用 handle，不代表最大已存在 handle 一定是 `HANDSEED-1`。

### 12.4 未知字段策略

Header 中存在多个只给出典型值的 unknown `BL/BS/B`。必须按版本消费其位宽并保留原值；不得因语义未知而跳过，否则后续位流全部错位。写入未知字段时优先 round-trip 原值；新建文件使用原规范明确给出的默认值。（pp.72–85）

<a id="dwg-classes-section"></a>
## 13. `AcDb:Classes`

### 13.1 用途

固定 object type 之外的实体/对象由 class table 动态编号。对象 type 大于等于 500 时，通常以 `type - 500` 得到 class-list 索引，再由 `classdxfname` 决定实际类型。必须验证索引范围；未知 class 应保存为 proxy/unknown object，不应丢弃。（pp.86–88, 109）

### 13.2 R13–R15

Beginning sentinel：

```text
8D A1 C4 B8 C4 A9 F8 C5 C0 DC F4 5F E7 CF B6 8A
```

随后 `RL class data size`，重复 class records，再 CRC/end sentinel。每条 class 包含 class number、proxy flags、app name、C++ class name、DXF name、zombie 标志和 entity/object class ID。

### 13.3 R2004+

R2007+ 字符串位于独立 string stream。Class record 的核心字段为：

| 字段 | 编码 | 含义 |
|---|---|---|
| class number | `BS` | 动态 type 基础编号 |
| proxy flags | `BS` | erase/transform/color/layer/linetype/visibility/cloning 等允许标志 |
| app/C++/DXF names | `TV`/`TU` | 注册应用、运行时类、DXF 类名 |
| was-a-zombie | `B` | proxy/zombie 状态 |
| item class ID | `BS` | `0x1F2` entity，`0x1F3` object |
| instance count | `BL` | 当前数据库创建数量，DXF 91 |
| DWG/maintenance version | `BL` | class 版本信息 |
| unknown fields | `BL` | 通常 0，原样保留 |

<a id="dwg-metadata-sections"></a>
## 14. 元数据与辅助 Sections

### 14.1 Padding 和空名称 Section

R13C3+ 在 Classes/Template 后可能有约 200 bytes padding，用于恢复或保持布局；存在 R13 Template 时长度相应变化。（p.89）

R2004+ 可有名称为空字符串的 section，属性为 compressed=2、encrypted=0、page size=`0x7400`。**未知：**原规范未确定其语义，只列字段和值。应按 opaque bytes 保留。（p.90）

### 14.2 `AcDb:SummaryInfo`

包含 title、subject、author、keywords、comments、last-saved-by、revision、hyperlink base，以及自定义 key/value properties。5.4.2 明确自定义属性即 `CUSTOMPROPERTYTAG`/`CUSTOMPROPERTY` 键值对（对应 DXF 1000 系列组码）：`Int16` property count，后随 `2 + 2*(2+n)` 的成对 key/value 字符串。文本在旧版本按 code page、R2007+ 按 Unicode。计数和字符串长度均须做 section 边界检查。（pp.91–92；5.4.2 修订）

### 14.3 `AcDb:Preview`

pre-R13C3 image 位于较早位置；R13C3+ 移到文件尾部附近并由 header seeker 指向。Preview 包装含 sentinel、总大小、多个图像条目；条目给出 type、address、size，常见为 BMP/WMF 等 preview。未知图像类型应按 size 跳过并保留。（pp.93–94）

### 14.4 `AcDb:VBAProject`

保存原始 VBA project bytes。`Encrypted=2` 的确切含义在规范中未知。Project data 是不透明负载；不要把内部内容当作 DWG bit stream 解析。处理不可信文件时不得执行宏。（p.95）

### 14.5 `AcDb:AppInfo`

保存 producer/version 相关字符串和校验信息。包装按 R2004 与 R2007+（或 class_version==3）分两种：R2004 为 16-bit 长度 + 单字节 0 结尾字符串，R2007+ 为 16-bit 长度 + UTF-16 宽字符（2 bytes/字符，0 结尾）。字段依次为：app info 名称（ACAD 写 `AppInfoData`，ODA 写 `AppInfoDataList`）、`RL` num strings（默认 0）、comment（如 `5004`，ODA 写 `4001`）、产品字符串（如 `Autodesk Architectural Desktop 2007`）、版本字符串（如 `5.0.318.0`，ODA 写 `2.7.2.0`），随后为 16 字节 version/comment/product checksum（ODA 与 LibreDWG 写零）。未知常量应保留。（pp.96–97；5.4.2 详表）

### 14.6 `AcDb:FileDepList`

保存 xref、字体、图像等外部文件依赖。由 entry count 和重复记录组成，记录通常含 feature、full path、found path、fingerprint/version 等字符串与 flags。Section 只描述依赖，不保证路径安全或文件存在；应用不得自动访问未验证的绝对路径、UNC 或父目录路径。（pp.98–99）

### 14.7 `AcDb:RevHistory`

保存 revision/save history 数值和时间数据；R2004 与 R2007 格式有变化。它不是对象图的权威来源，损坏时可作为可选元数据忽略。（p.100）

### 14.8 `AcDb:Security`

包含 security flags、provider/name、加密相关参数和签名/证书类 bytes。ODA 规范对完整安全算法描述有限。读取器可展示和保留元数据，但不得声称已验证数字签名，除非实现了对应密码学协议和证书验证。（pp.101–102）

### 14.9 `AcDb:ObjFreeSpace`

保存对象空闲空间和若干 handle/size 统计。原规范认为其含义部分未知。读取不应依赖它定位有效对象；Object Map 才是定位依据。（p.249）

### 14.10 `AcDb:Template`

包含 measurement（英制/公制）和少量模板/描述字段。R13 与 R14–R15 所在物理位置不同。它是可选 section。（p.250）

### 14.11 `AcDb:AuxHeader`

R2000 引入的冗余辅助头，包含内部 DWG version 数字（AC1012=19 至 AC1032=33）、maintenance version、保存次数拆分值、creation/update datetime、HANDSEED 截断值、educational plot stamp 和大量固定/零字段。R2018 尾部扩展零值字段。主 header 有效时 AuxHeader 仅作交叉验证；冲突应报告而非静默覆盖。（pp.267–268）

### 14.12 Unknown Section（旧格式）

R13C3–R15 locator record 3 指向 53-byte unknown section。规范写入策略是：`RL 0`、`RL(object-map entries + 3)`、45 bytes 已知有效常量，并在 offset 16 patch object-data start address。该结构语义未知；读取时保留完整 53 bytes，写入兼容文件时只能使用规范常量或原样 round-trip。（p.263）

<a id="dwg-objects-section"></a>
## 15. `AcDb:AcDbObjects` 与对象记录

### 15.1 Section 属性和语义

典型属性：name=`AcDb:AcDbObjects`、compressed=2、encrypted=0/1、page size=`0x7400`。R2004+ section data 开头可有 `RL 0x0DCA`，语义未知。对象可任意排序，可能有不可达对象、重复 handle 或重复名称。（p.103）

<a id="dwg-object-record"></a>
### 15.2 非实体对象通用记录

逻辑顺序如下；版本条件决定字段是否存在：

```text
MS object byte size（不含 CRC）
[R2010+] MC handle-stream bit size（unsigned；0x40 不作符号）
OT object type
[R2000–R2007] RL endbit / data bits before handles
H own handle
BS EED size + EED blocks，直到零长度
[R13–R14] RL object-data bit size
reactor count / xdictionary-missing flag / DataStorage flag
type-specific data fields
[R2007+] optional string stream + trailing encoded string size
handle stream：owner、reactors、xdictionary、type-specific handles
padding to byte boundary
RS CRC-16
```

R2007+ string stream 的存在 bit 位于 pre-handles section 最后。若存在，从 endbit 向前读 16-bit `strDataSize`；若其 `0x8000` 置位，再向前读高位 size，并组合为大长度，然后反向得到 string stream start。必须先确定 data/handle endbit，才能建立三个独立 reader。（pp.103–104）

<a id="dwg-entity-record"></a>
### 15.3 实体通用记录

实体在对象通用头之后增加：

| 字段 | 编码 | 版本/条件 | 含义 |
|---|---|---|---|
| graphic-present | `B` | common | 是否内嵌 proxy graphic |
| graphic size | `RL`/`BLL` | R13–R2007 / R2010+ | graphic byte size |
| entity mode | `BB` | common | owner/model/paper-space 模式 |
| reactor count | `BL` | common | persistent reactor 数 |
| xdictionary missing | `B` | R2004+ | 1 表示无 xdictionary handle |
| has DS binary | `B` | R2013+ | DataStorage 中有关联 binary data |
| no-links | `B` | R2000+ | previous/next 是否采用默认 ±1 |
| color | `CMC`/`ENC` | common/版本 | 实体颜色 |
| linetype scale | `BD` | R2000+ | 线型缩放 |
| linetype flags | `BB` | R2000+ | bylayer/byblock/continuous/handle |
| plotstyle flags | `BB` | R2000+ | 同类枚举 |
| material flags | `BB` | R2007+ | 同类枚举 |
| shadow flags | `RC` | R2007+ | 阴影模式 |
| visual-style flags | `B×3` | R2010+ | full/face/edge style handles 是否存在 |
| invisibility | `BS` | common | bit 0 表示不可见 |
| lineweight | `RC` | R2000+ | lineweight enum |

Handle stream 随后包含 owner、reactors、xdictionary、layer、可选 linetype、previous/next、color-book color、material、plotstyle 和 visual-style handles。是否存在由前述 flags 决定，顺序不可交换。（pp.105–111）

### 15.4 R13–R14 公共 flags

旧格式以 6-bit `FEDCBA` 表示 entity mode、reactor/linetype/linker 等条件，再以 6-bit `CCSSII` 表示 color `BS`、linetype scale `BD`、invisible `BS` 的压缩存在形式。它们不能按 R2000+ 字段序列读取。（pp.106–107）

<a id="dwg-object-type-codes"></a>
## 16. 固定 Object Type 目录

| Code | 类型 | Code | 类型 |
|---:|---|---:|---|
| `0x01` | TEXT | `0x02` | ATTRIB |
| `0x03` | ATTDEF | `0x04` | BLOCK |
| `0x05` | ENDBLK | `0x06` | SEQEND |
| `0x07` | INSERT | `0x08` | MINSERT |
| `0x0A`–`0x0E` | VERTEX 2D/3D/MESH/PFACE/PFACE FACE | `0x0F`–`0x10` | POLYLINE 2D/3D |
| `0x11` | ARC | `0x12` | CIRCLE |
| `0x13` | LINE | `0x14`–`0x1A` | DIMENSION variants |
| `0x1B` | POINT | `0x1C` | 3DFACE |
| `0x1D` | POLYLINE PFACE | `0x1E` | POLYLINE MESH |
| `0x1F` | SOLID | `0x20` | TRACE |
| `0x21` | SHAPE | `0x22` | VIEWPORT entity |
| `0x23` | ELLIPSE | `0x24` | SPLINE |
| `0x25` | REGION | `0x26` | 3DSOLID |
| `0x27` | BODY | `0x28` | RAY |
| `0x29` | XLINE | `0x2A` | DICTIONARY |
| `0x2B` | OLEFRAME | `0x2C` | MTEXT |
| `0x2D` | LEADER | `0x2E` | TOLERANCE |
| `0x2F` | MLINE | `0x30`/`0x31` | BLOCK CONTROL/HEADER |
| `0x32`/`0x33` | LAYER CONTROL/LAYER | `0x34`/`0x35` | STYLE CONTROL/STYLE |
| `0x37`/`0x38` | LTYPE CONTROL/LTYPE | `0x3C`/`0x3D` | VIEW CONTROL/VIEW |
| `0x3E`/`0x3F` | UCS CONTROL/UCS | `0x40`/`0x41` | VPORT CONTROL/VPORT |
| `0x42`/`0x43` | APPID CONTROL/APPID | `0x44`/`0x45` | DIMSTYLE CONTROL/DIMSTYLE |
| `0x46`/`0x47` | VP ENT HDR CONTROL/HEADER | `0x48` | GROUP |
| `0x49` | MLINESTYLE | `0x4A` | OLE2FRAME |
| `0x4C` | LONG_TRANSACTION | `0x4D` | LWPOLYLINE |
| `0x4E` | HATCH | `0x4F` | XRECORD |
| `0x50` | ACDBPLACEHOLDER | `0x51` | VBA_PROJECT |
| `0x52` | LAYOUT | `0x1F2`/`0x1F3` | ACAD_PROXY_ENTITY/OBJECT |

空缺 code 是保留或版本相关值。动态 class objects 的 type 通常从 500 起，通过 Classes section 解析。（pp.107–109）

<a id="dwg-entity-prescriptions"></a>
## 17. 实体字段规格

本章字段均位于第 15 章公共实体 data 之后，相关 handles 位于公共 handles 之后。`count × type` 表示按 count 重复；所有 count 先验证剩余 section/object 大小。

### 17.1 文本和属性

| 类型 | 关键 data 字段（逻辑顺序） | 关键 handles/版本说明 | 来源 |
|---|---|---|---:|
| TEXT | data flags、elevation、insertion/alignment 2D points、extrusion、thickness、oblique、rotation、height、width factor、text value、generation/halign/valign | text style handle；`DD` 大量使用默认 insertion point | pp.111–116 |
| ATTRIB | TEXT 字段 + tag、field length、attribute flags、lock position、mtext/annotative 数据 | style 和 version-specific context handles | pp.116–120 |
| ATTDEF | ATTRIB 字段 + prompt | 同 ATTRIB；R2018 有变化 | pp.120–123 |
| MTEXT | insertion、extrusion、x-axis、rectangle width/height、text height、attachment/drawing direction、extents、text、line spacing、background fill、columns | style handle；R2018 column 字段变化 | pp.153–157 |
| TOLERANCE | height、dim gap、insertion、x direction、extrusion、text | dimstyle handle | pp.172–173 |

### 17.2 基础几何

| 类型 | 关键字段 | 条件 |
|---|---|---|
| POINT | `3BD point`、`BT thickness`、`BE extrusion`、x-axis angle | angle 影响 display orientation |
| LINE | z-is-zero flag、start/end（`RD/DD` 组合）、`BT`、`BE` | 2D/3D 压缩路径不同 |
| CIRCLE | `3BD center`、`BD radius`、`BT`、`BE` | radius 应非负 |
| ARC | CIRCLE 字段 + start/end angle | angle 为弧度 |
| ELLIPSE | center、major-axis vector、extrusion、axis ratio、start/end parameters | ratio/参数需有限值 |
| RAY/XLINE | base point + direction vector | direction 不应为零向量 |
| SOLID/TRACE | thickness、elevation、四个 2D corners、extrusion | SOLID 与 TRACE 顶点顺序语义不同 |
| 3DFACE | 四个 3D corners、invisibility flags | DD 默认复用前点分量 |

（pp.126–152, 163–165）

### 17.3 Polyline 家族

- VERTEX 2D：flags、point、start/end width、bulge、tangent direction、vertex ID。
- VERTEX 3D/MESH：flags + `3BD point`。
- VERTEX PFACE：point；PFACE FACE 保存四个 vertex indexes 和 edge invisibility。
- POLYLINE 2D/3D/MESH/PFACE：flags、curve/smooth type、default widths、m/n counts/densities，以及 first/last vertex 或 owned-vertex handles。
- LWPOLYLINE：flags 决定 constant width、elevation、thickness、normal、point count、bulge count、vertex ID count、width count；坐标首点通常完整 `2RD/2BD`，后续以 `DD` 相对前点压缩。各可选数组数量必须和 point count/flags 交叉验证。

（pp.123–140, 213–214）

### 17.4 Dimensions

所有 dimension 先读 common dimension data：extrusion、text midpoint、elevation、flags、user text、text rotation、horizontal direction、insertion scale/rotation、attachment/line spacing、measurement、version-specific fields，并引用 dimstyle 和 anonymous block。随后按类型读取 definition points：ordinate、linear、aligned、angular 2-line、angular 3-point、radius、diameter、large radial/arc dimension 各自点和 leader length/angles。（pp.126–135）

### 17.5 SPLINE

字段包括 scenario（fit points 或 knots/control points）、degree、rational/closed/periodic flags、knot/control tolerances、counts、knots、control points、weights、fit tolerance、fit points及首尾 tangent。Counts 受 degree 和 scenario 约束；读取器必须拒绝导致巨大分配或乘法溢出的组合。（pp.165–168）

### 17.6 HATCH

包含 elevation、extrusion、pattern name、solid/associative flags、boundary path count。每条 path 有 flags，并按 polyline boundary 或 edge list 分支；edge 可为 line、circular arc、elliptic arc、spline。随后是 source boundary handles、style/pattern type、angle/scale/double、pattern definition lines、seed points和 gradient 数据。Spline edge 内部再含 degree、rational/periodic、knots、control points、weights和 fit data。所有嵌套 count 必须受 object size 限制。（pp.193–198）

### 17.7 VIEWPORT、IMAGE 与裁剪

- VIEWPORT entity：center/width/height、view target/direction、twist、view height、lens、front/back clip、render mode、UCS、snap/grid、frozen layers、visual style、lighting 和大量 flags/handles。
- IMAGE：class version、insertion/u/v vectors、pixel size、display/clipping flags、brightness/contrast/fade、clip boundary vertices；handles 指向 IMAGEDEF 和 reactor。
- SPATIAL_FILTER：clip boundary points、origin、normal、front/back clip、inverse/forward transform matrices。
- WIPEOUT 通常沿用 raster image 类字段和 class registration。

（pp.141–146, 204–207, 220–224）

### 17.8 REGION、3DSOLID、BODY

R2000–R2007 可在对象 data 中保存 ACIS SAT 文本块；后续版本可用 SAB/ASM binary，并在 R2013+ 通过 `has DS binary data` 与 DataStorage 关联。记录还包含 version、acis-empty、wireframe/silhouette 数据、history handle 等版本字段。SAT/SAB 内部几何是独立格式，不能按 DWG bitcodes 读取。（pp.168–172, 252–259）

### 17.9 LEADER、MLEADER、MLINE

- LEADER：annotation type/path type、points、origin/normal/horizontal direction、offsets、dimension flags，并引用 annotation、dimstyle和 block。
- MLEADER：context data（scale、content、leader roots/lines、breaks、text/block content）、overrides 和 style handle；版本分支复杂，应由 class version 驱动。
- MLINE：scale、justification、base point、extrusion、vertices；每 vertex 含 direction/miter 和按 MLINESTYLE element count 的参数/area fill arrays。

（pp.157–163, 173–181）

### 17.10 TABLE 系列

R2007 及以前 TABLE 有独立实体字段；R2010+ 大量内容迁入 `AcDbTableContent`、cell styles 和 value/custom-data collections。5.4.2 明确：R2010 的 DWG 文件**总是**使用新的 `AcDbTableContent` 结构（DXF 可同时保留旧结构），表实体通过 handle 指向 content 对象；`AcDbTableContent` 早在 R2007 已引入，但经表格扩展字典 `ACAD_XREC_ROUNDTRIP` 间接引用（TODO：2007 的 roundtrip 细节）。核心结构包括 rows/columns、cell types/flags、文本或 block content、merge ranges、边框、背景/内容颜色、alignment、margins、row/column sizes、field/block handles。TABLESTYLE 同样分 R2007- 和 R2010+ 两套格式；R2010 起 cell style ID 1=title、2=header、3=data、4=table（`CELLSTYLEMAP` 可含自定义 cell styles）。解析必须按 class/version 明确分支，禁止把旧 cell record 和新 content record 混读。（pp.229–247；5.4.2 修订）

<a id="dwg-nonentity-objects"></a>
## 18. 非实体对象、表和控制对象

### 18.1 Control object 通则

BLOCK/LAYER/STYLE/LTYPE/VIEW/UCS/VPORT/APPID/DIMSTYLE control objects 保存 record 数量和 record handles。Control 本身通常由 Header 根 handle 引用；records 的 owner 应回指 control。数量与实际 handle 数不一致时，以安全可读取边界为限并报告损坏。（pp.181–193）

### 18.2 Symbol-table records

| 对象 | 关键字段 |
|---|---|
| BLOCK_HEADER | name、xref/overlay/loaded flags、base point、xref path、description、preview、insert count、owned entities、layout handle |
| LAYER | name、flags、color、plot flag、lineweight；linetype、plotstyle、material handles |
| STYLE | name、flags、fixed height、width factor、oblique、generation flags、last height、font/bigfont |
| LTYPE | name、description、pattern length、dash count；每 dash 的 length、shape flags、offset/scale/rotation/text/shape handles |
| VIEW | name、view size/center/target/direction、twist/lens/clips、render/UCS flags、background/visual-style handles |
| UCS | name、origin、x/y direction、elevation、orthographic origins、base UCS handle |
| VPORT | name、view/snap/grid/UCS/render/lighting fields和 handles |
| APPID | name + standard flags |
| DIMSTYLE | 大量 `DIM*` 变量、颜色/文本/几何设置；style/linetype handles |

### 18.3 DICTIONARY 与 XRECORD

DICTIONARY 保存 entry count、cloning/ownership flags、key strings 和 value handles；DICTIONARYWDFLT 另有 default entry handle。Key 与 handle 数必须一致，重复 key 应诊断。

XRECORD 保存 cloning flags 和以 DXF group-code typed values 表示的任意数据，再保存 object references/reactors。5.4.2 将 XRECORD 标为 `(varies)`：其字段 prescription 依赖 class version，解析前必须先解析 class。未知 group code 必须按其已知值宽度或受限 opaque 形式保留，不能猜长度后继续。（pp.152–153, 247–248；5.4.2 修订）

### 18.4 其他动态对象

| 类别 | 类型 |
|---|---|
| 上下文/注释比例 | `AcDbObjectContextData`、`AcDbAnnotScaleObjectContextData`、`MLeaderAnnotContext`、`SCALE` |
| 显示/材质 | `MATERIAL`、`VISUALSTYLE`、`DBCOLOR`、`RASTERVARIABLES` |
| 布局/打印 | `LAYOUT`、`PLOTSETTINGS` |
| 排序/索引 | `SORTENTSTABLE`、`LAYER_INDEX`、`SPATIAL_INDEX`、`IDBUFFER` |
| 字段 | `FIELD`、`FIELDLIST` |
| 表格 | `TABLECONTENT`、`TABLESTYLE`、`CELLSTYLEMAP`、`TABLEGEOMETRY` |
| 图像 | `IMAGEDEF`、`IMAGEDEFREACTOR`、`WIPEOUTVARIABLE` |
| 地理 | `GEODATA` |
| 参数化实体 | `ACSH_BOX/WEDGE/SPHERE/CYLINDER/CONE/PYRAMID/FILLET/CHAMFER/TORUS/BREP/BOOLEAN/HISTORY_CLASS`（5.4.2 新增 prescription，见 20.4.107–118） |
| 光照 | `SUN`（viewport 的 hard-owned 子对象，非实体；5.4.2 新增，见 20.4.119） |
| pre-R13 | `REPEAT`(5)、`ENDREP`(6)、`3DLINE`(21)、`JUMP`(10)、`LOAD`(10)（5.4.2 新增，见 20.4.120–124） |
| 代理/占位 | `ACAD_PROXY_OBJECT`、`PLACEHOLDER` |

这些类型通常由 Classes section 的 `classdxfname` 识别，而不是固定 type code。原规范 pp.198–248 给出各自字段 prescription；AI 查询具体类型时，应同时检索类型名、class version 和其 common object format。

#### 18.4.1 `AcDbEvalExpr` 与 `AcDbShHistoryNode` 子结构（5.4.2 新增）

`AcDbEvalExpr`（20.4.105）：`parentid BL`、`major version BL`（默认 33）、`minor version BL`（默认 29）、`value_code BS`（下一值的 DXF 组码，按 40/10/11/1/90/91/70 分派为 BD/2RD/3RD/T/BL/H/BL）、`nodeid BL`。

`AcDbShHistoryNode`（20.4.106）：`major version BL`（见 27–33）、`minor version BL`（见 29–106）、`trans 16×BD`（变换矩阵）、`color CMC`、`step_id BL`、`material H`。

12 个 `ACSH_*_CLASS`（20.4.107–118）均为动态 class（≥500，R2000+，proxy flags 499），每个都内嵌 `AcDbEvalExpr` + `AcDbShHistoryNode`，随后是该类几何参数（BOX/WEDGE：length/width/height；SPHERE：radius；CYLINDER/CONE：height/radius；PYRAMID：height/length/width；FILLET/CHAMFER：对应半径/距离；TORUS：major/minor radius；BREP/BOOLEAN/HISTORY：acis/历史字段）。

<a id="dwg-type-field-index"></a>
## 19. 全类型字段检索索引

本索引覆盖原规范全部 `20.4.3–20.4.104` 条目。每项先读取 common entity/object data，再读取“主体字段”，最后在 handle stream 读取“引用”。未列出的 example hex dump 仅用于验证编码，不改变字段语义。

| 原节 | 类型 | 主体字段/数据组 | 主要引用或说明 |
|---:|---|---|---|
| 20.4.3–5 | TEXT / ATTRIB / ATTDEF | text geometry、content、alignment；ATTRIB 加 tag/flags，ATTDEF 加 prompt | STYLE；见 17.1 |
| 20.4.6–8 | BLOCK / ENDBLK / SEQEND | name、block flags 或空主体 | BLOCK_HEADER owner；SEQEND 结束 owned 序列 |
| 20.4.9–10 | INSERT / MINSERT | insertion、scale flags/values、rotation、extrusion、attributes flag；MINSERT 加 rows/cols/spacings | BLOCK_HEADER、first/last ATTRIB 或 owned handles、SEQEND |
| 20.4.11–15 | VERTEX variants | flags、point、width/bulge/tangent 或 face indexes | POLYLINE owner |
| 20.4.16–17 | POLYLINE 2D/3D | flags、curve type、widths、extrusion | vertex sequence、SEQEND |
| 20.4.18–21 | ARC / ARC_DIMENSION / CIRCLE / LINE | center/radius/angles 或 endpoints、thickness、extrusion | 公共 entity handles |
| 20.4.22–30 | DIMENSION family | common dimension + 各类 definition points/angles/leader length | DIMSTYLE、anonymous BLOCK；见 17.4 |
| 20.4.31–36 | POINT / 3DFACE / PFACE/MESH POLYLINE / SOLID / TRACE | 点、网格 counts、corners、flags | vertices/SEQEND；见 17.2–3 |
| 20.4.37 | SHAPE | insertion、size、rotation、width factor、oblique、thickness、shape number、extrusion | STYLE/SHAPEFILE |
| 20.4.38 | VIEWPORT ENTITY | view、clip、UCS、snap/grid、render/light、frozen layers | frozen layers、style、background、sun等 |
| 20.4.39–40 | ELLIPSE / SPLINE | conic parameters 或 spline scenario/arrays | 公共 handles |
| 20.4.41 | REGION / 3DSOLID / BODY | ACIS SAT/SAB、wire/silhouette/history flags | history、DataStorage；见 17.8 |
| 20.4.42–43 | RAY / XLINE | base + direction | 公共 handles |
| 20.4.44–45 | DICTIONARY / DICTIONARYWDFLT | count、cloning/ownership flags、keys | value handles、default handle |
| 20.4.46 | MTEXT | text frame、content、spacing、background、columns | STYLE |
| 20.4.47–50 | LEADER / MLEADER / TOLERANCE / MLINE | points、annotation context、style flags、MLINE vertex arrays | annotation/style/block/material 等 |
| 20.4.51–52 | BLOCK CONTROL / HEADER | record count；block metadata、base/xref/preview、owned entities | model/paper space、records、layout |
| 20.4.53–54 | LAYER CONTROL / LAYER | count；name、flags、color、plot/lineweight | layer records；linetype/plotstyle/material |
| 20.4.55–56 | SHAPEFILE CONTROL / SHAPEFILE(STYLE) | count；font style fields | style records |
| 20.4.57–58 | LINETYPE CONTROL / LTYPE | count；name/description/pattern/dashes | ltype records；complex dash style handles |
| 20.4.59–60 | VIEW CONTROL / VIEW | count；view geometry、UCS/render/background | view records、UCS/background/style/sun |
| 20.4.61–62 | UCS CONTROL / UCS | count；origin、axes、ortho origins | UCS records、base UCS |
| 20.4.63–64 | VPORT CONTROL / VPORT | count；view/snap/grid/UCS/render fields | VPORT records、background/style/sun |
| 20.4.65–66 | APPID CONTROL / APPID | count；name、flags | APPID records |
| 20.4.67–68 | DIMSTYLE CONTROL / DIMSTYLE | count；完整 DIM* 参数集 | dimstyle records、text/arrow/linetype handles |
| 20.4.69–70 | VP ENT HDR CONTROL / HEADER | count；viewport entity header flags | records、viewport entities |
| 20.4.71 | AcDbAnnotScaleObjectContextData | class version、scale context fields | annotation scale |
| 20.4.72 | GROUP | name/description、unnamed/selectable flags、count | grouped entity handles |
| 20.4.73 | MLINESTYLE | name/description、flags、fill、start/end angles、elements | per-element linetype index；R2018+ handle |
| 20.4.74 | DICTIONARYVAR | schema/version flag + value | dictionary-owned object |
| 20.4.75 | HATCH | boundary paths、pattern、seeds、gradient | source boundary handles；见 17.6 |
| 20.4.76–77 | FIELD / FIELDLIST | field evaluator/code/state/value/child count；list count | child/owner/associated object handles |
| 20.4.78 | GEODATA | coordinate system、design/reference points、units、mesh points/faces、XML/WKT text | block/table owner |
| 20.4.79 | IDBUFFER | count | repeated object handles |
| 20.4.80–82 | IMAGE / IMAGEDEF / IMAGEDEFREACTOR | image geometry/clip；pixel/file/load fields；class version | image definition/reactor/image handles |
| 20.4.83 | LAYER_INDEX | timestamp、entry count、layer names/IDs | layer handles |
| 20.4.84 | LAYOUT | plot settings + layout name/tab/limits/base/UCS/extents/elevation | paperspace block/record、viewport、UCS handles |
| 20.4.85 | LWPLINE | conditional scalar fields、points、bulges、IDs、widths | 公共 handles；见 17.3 |
| 20.4.86–87 | MLeaderAnnotContext / MLEADERSTYLE | annotation context；style defaults、content/leader/landing/text/block settings | style、text style、block、arrow、linetype handles |
| 20.4.88 | OLE2FRAME | type/version、length、binary OLE data | opaque payload；规范无 example |
| 20.4.89 | AcDbObjectContextData | class/version/context fields | context collection/scale |
| 20.4.90 | PROXY | class ID、proxy flags、original format、data bytes、object IDs | proxy object/entity handles |
| 20.4.91 | RASTERVARIABLES | class version、frame、quality、units | dictionary owner |
| 20.4.92 | SCALE | name、paper units、drawing units、unit-scale flag | annotation dictionaries |
| 20.4.93 | SORTENTSTABLE | sort handle count | entity handles + sort handles |
| 20.4.94 | SPATIAL_FILTER | clip boundary、normal/origin、clip planes、matrices | spatial-filter ownership |
| 20.4.95 | SPATIAL_INDEX | timestamp/version + opaque index bytes | **未知：**索引内部结构有限 |
| 20.4.96–100 | TABLE / TABLECONTENT / Cell geometry / Value / Custom data | table dimensions、cells、content records、typed values、key/value custom data | field/block/style handles；按 R2007-/R2010+ 分支 |
| 20.4.101 | TABLESTYLE | style name/description、flow/margins、row/cell styles、borders/content formats | text styles、blocks；版本分支 |
| 20.4.102 | CELLSTYLEMAP | style entry map | table style ecosystem |
| 20.4.103 | TABLEGEOMETRY | cell geometry records、positions/sizes | table/entity references |
| 20.4.104 | XRECORD | cloning flags、typed value list | object references/reactors |
| 20.4.105 | AcDbEvalExpr | parentid、major/minor version、value_code 分派、nodeid | 内嵌于 ACSH/SUN 等动态对象（5.4.2） |
| 20.4.106 | AcDbShHistoryNode | major/minor version、16×BD 矩阵、CMC、step_id、material | 内嵌于 ACSH 对象（5.4.2） |
| 20.4.107–118 | ACSH_*_CLASS | 参数化实体几何字段（按类变化） | AcDbEvalExpr + AcDbShHistoryNode（5.4.2） |
| 20.4.119 | SUN | class_version、is_on、color、intensity、has_shadow、julian/msecs、is_dst、shadow 参数 | viewport 的 hard owner（5.4.2） |
| 20.4.120–121 | REPEAT/ENDREP（pre-R2.1） | 无字段 / numcols、numrows、spacing | 块式重复实体对（5.4.2） |
| 20.4.122 | 3DLINE（R2.4–R11） | start/end（2RD/3RD 按版本）、extrusion | 公共实体（5.4.2） |
| 20.4.123 | JUMP（pre-R13） | jump_entity_section、jump_address（3×RC） | EXTRAS 跳转（5.4.2） |
| 20.4.124 | LOAD（pre-R2.0） | 公共实体数据 | 仅旧版（5.4.2） |

对标为 `(UNDOCUMENTED)` 的 control objects，原规范仍给出逆向字段 prescription；“undocumented”指 Autodesk 未公开，不等于 ODA 文档完全没有结构。对 `varies` 类型必须先通过 Classes section 解析 class number。

### 19.1 原规范标题别名

以下名称保留原文大小写与拼写，供 AI 按 PDF 标题精确检索；其字段规格位于第 15–19 章：

- `Common Entity Data` → 15.3；`Common Entity Handle Data` → 15.3 handle stream。
- `2D POLYLINE`、`3D POLYLINE` → 17.3。
- `COMMON DIMENSION DATA`、`LARGE_RADIAL_DIMENSION` → 17.4。
- `BLOCK HEADER` → 18.2；`VIEWPORT ENTITY CONTROL` → 18.1 与类型索引。
- `Cell content geometry`、`Custom data collection` → 17.10 与本章 TABLE 系列条目。

<a id="dwg-object-map"></a>
## 20. `AcDb:Handles` — Object Map

### 20.1 R13–R15

Object Map 是 handle 到物理对象位置的有序增量表。初始化 `last_handle=0`、`last_location=0`，重复读取 chunks：

```text
BE UInt16 chunk_size（唯一明确的大端字段之一）
repeat until chunk payload exhausted:
    MC handle_delta       // 非负，handle 递增
    MC location_delta     // 可负
CRC-16, high byte first
```

Chunk 最大长度 2032 bytes。最后一个空 chunk 只有 2-byte CRC，即 `chunk_size == 2`。对每项累加 delta 得到绝对 handle 和 file location。（p.251）

### 20.2 R2004+

R2004 Object Map 位于压缩的 `AcDb:Handles` logical section，解压后记录布局与旧格式基本一致，但 location 是相对 `AcDb:AcDbObjects` 逻辑 section 起点的 offset，不是物理文件绝对地址。对象可跨物理 pages，因此必须先重建 logical section 再定位。（p.251）

### 20.3 验证规则

- handle delta 必须保持严格递增或至少不回退；重复 handle 为损坏/冲突；
- location 可因对象排列出现负 delta，但解析后的地址必须落在 logical objects section；
- Object Map 的 size 指向对象记录起点，具体记录长度仍从对象 `MS` 读取；
- CRC 错误时不得无边界扫描整个文件来猜对象，除非显式进入恢复模式。

<a id="dwg-datastorage"></a>
## 21. R2013+ `AcDb:AcDsPrototype_1b` DataStorage

### 21.1 总体结构

DataStorage 是 byte stream，不是 bit stream。它由 file header、多个 file segments，以及 segment/schema/data/search indexes 构成。主要用于 REGION/3DSOLID 等 ACIS 对象的 SAB/ASM bytes。（p.252）

```text
DataStore Header
Segment Index (segidx)
Data Index (datidx)
Data segments (_data_)
Blob pages (blob01)          // 大数据
Schema Index (schidx)
Schema Data (schdat)
Search segment (search)
padding
```

### 21.2 DataStore Header

包含 signature、header size、version/revision、segment-index offset/count、schema/data/search segment indexes、previous-save index 和 file size。多个字段原规范标为 always 0/2 的 unknown；应保留并验证总大小。（p.252）

### 21.3 Segment Header

Segment signature 通常 `0xD5AC`，name 为 6 bytes（`segidx`、`datidx`、`_data_`、`schidx`、`schdat`、`search`、`blob01`），再含 segment index、size、revision、system/object alignment offsets 和固定 alignment bytes。Segment 总长度为 `0x40` 的倍数，AutoCAD 通常写成 `0x80` 的倍数，以 `0x70` bytes padding。（p.253）

### 21.4 索引与数据记录

- Segment Index：重复 `UInt64 offset + UInt32 size`，offset 相对 DataStore 开头。
- Data Index：entry count；每项 `segment index + local offset + schema index`。
- Data Segment：按 data-index local offsets 找 headers；header 含 entry size、handle、record local offset。小于等于约 `0x40000` bytes 的 payload 可直接存储。
- 大 payload：以 marker `0xBB106BB1` 表示 blob reference，含 total size、page count/size、last-page size和各 `blob01` segment indexes。
- Blob01：total size、page start、page index/count、page data size、raw bytes。

（pp.254–256）

### 21.5 ACIS/SAB payload

每个 ACIS entity 对应一个 data record。SAB bytes 前缀为 ASCII `ACIS BinaryFile`；由 SAT 转 SAB 时，version >= 21800 的尾标记为 `End-of-ASM-data`，更旧为 `End-of-ACIS-data`。这些 marker 是 payload 内部验证线索，不是 DataStore segment 边界。（p.254）

### 21.6 Schema 和 Search

Schema Index 指向 Schema Data 中的 property/schema；schema 是 name-indexed typed properties，type 0–15 决定固定或 custom size。Search segment 把 sorted indexes 与 object handles 关联。原规范称 search 数据目的部分未知，读取时可忽略；但写入时若完全不生成，AutoCAD 可能忽略相应 entity，因此兼容写入必须生成一致 search entries。（pp.256–259）

<a id="dwg-eed"></a>
## 22. Extended Entity/Object Data（EED）

EED 紧随对象 own handle。重复结构：

```text
BS application_block_length
if length == 0: end
H  application APPID handle（标准 table handle reference）
typed data items，恰好消耗 length
```

Item 首字节 code=`DXF group code - 1000`：

| code | DXF | 值 |
|---:|---:|---|
| 0 | 1000 | R13–R2004：`RC len + RS codepage + len bytes`；R2007+：`RS char count + UTF-16` |
| 1 | 1001 | 规范认为在此无效/冗余 |
| 2 | 1002 | `RC`，0=`{`，1=`}` |
| 3 | 1003 | 8-byte layer handle value |
| 4 | 1004 | `RC length + binary bytes` |
| 5 | 1005 | 8-byte entity handle value |
| 10–13 | 1010–1013 | `3RD` point/vector |
| 40–42 | 1040–1042 | `RD` real |
| 70 | 1070 | `RS` integer |
| 71 | 1071 | `RL` integer |

EED block length按 bytes 计，不含自身 length 和 APPID handle。必须在该长度内解析，未知 item code 不得越过 block 猜测。（p.269）

<a id="dwg-proxy-graphics"></a>
## 23. Proxy Entity Graphics

Entity common data 中 graphic-present bit 为 1 时，后跟 graphic byte size（R13–R2007 `RL`，R2010+ `BLL`）和 graphic chunks。Graphic data 按 4-byte 对齐；`PS` 是 code-page zero-terminated padded string，`PUS` 是 UTF-16 双零终止 padded string。（pp.105, 270）

每个 chunk：`RL size | RL type | type-specific data`。已知 types 包括 extents、circle、3-point circle、arc、polyline、polygon、mesh、shell、text/text2、xline、ray，以及改变后续 primitive traits 的 subentity color/layer/linetype/marker/fill/truecolor/lineweight/ltscale/thickness/plotstyle/material/mapper。还有 PUSH/POP clip、model transform 和 LWPOLYLINE payload。（pp.270–278）

Mesh/Shell 的 edge/face/vertex primitive flags 决定随后颜色、layer、linetype、marker、visibility、normal 和 orientation arrays 是否存在。数组元素数由 rows/cols 或 face-entry topology 派生；必须先做安全乘法。未知 chunk type 可按 chunk size 跳过并保留。

<a id="dwg-read-pipeline"></a>
## 24. 规范化读取流程

1. 读取前 6 bytes，选择准确版本分支；拒绝未知签名或转入显式实验模式。
2. 解析固定 file header，验证所有 offset/size 算术。
3. R13–R15：读取 section locators；R2004+：恢复 Page Map 和 Section Map。
4. 对 R2007 system pages 依次做 RS decode、CRC、解压、重复数据验证；对 data pages 做对应校验。
5. 按 logical offsets 重建命名 sections，补全合法 zero pages，不允许重叠写出 logical bounds。
6. 解析 Header 和 Classes；建立 root handles、版本上下文、code page、dynamic class map。
7. 解压并解析 Object Map，得到 `handle → logical object offset`。
8. 对每个目标对象先建立 bounded object slice，再切分 data/string/handle streams。
9. 解析 common header，再按 fixed type 或 class DXF name 解析 type-specific fields。
10. 从 Header/control objects 开始遍历 ownership/pointer 图；标记 unreachable、duplicate 和 dangling handles。
11. 若 entity 有 DataStorage flag，按 object handle 和 indexes 关联 SAB/ASM payload。
12. 解析可选 metadata、EED 和 proxy graphics；保留所有未知字段和 opaque payload。
13. 汇总 CRC、边界、版本、重复名称和引用诊断，不把容错读取悄悄报告为完全有效。

<a id="dwg-write-pipeline"></a>
## 25. 规范化写入流程

1. 固定目标 DWG version/maintenance version，冻结对应字段 prescription。
2. 规范化数据库对象图：唯一 handles、有效 owners、control records、root dictionaries。
3. 分配 handles；HANDSEED 大于所有新分配 handle。
4. 序列化对象：先 data/string/handle 三个逻辑流，计算 endbit/stream sizes，加入 padding 和 object CRC。
5. 按 handle 排序构造 Object Map 增量 chunks。
6. 构造 Header、Classes、metadata sections；R2013+ 构造 DataStorage indexes、segments、schemas/search。
7. 按目标容器切分 pages；只有压缩结果更短时使用压缩形式。
8. 计算 page checksum/CRC，执行必要 RS encoding、interleaving、random padding 或 header encryption。
9. 构造 maps 和 file header；涉及自引用 offset 时使用受控回填。
10. 写 second/redundant headers 和 preview；刷新 file size、map addresses 和 CRC。
11. 重新以独立 reader 读取输出，验证所有对象可达性和 section/page integrity。

写入器不应把未知字段统一清零：对读后写 round-trip 应保留原值；对新建文件只使用原规范明确默认值。未知对象应尽量保留原始 record，并在 handle 重映射会破坏其内部引用时拒绝无损承诺。

<a id="dwg-validation"></a>
## 26. 验证、容错与安全要求

### 26.1 分层错误

| 层 | 典型错误 | 建议行为 |
|---|---|---|
| Signature | 未知 AC code | 停止或实验模式 |
| Container | map/page offset 越界、重叠 | 严格模式停止；恢复模式隔离 section |
| Codec | LZ offset 越界、尺寸不符 | 停止该 page，禁止部分输出冒充完整数据 |
| Integrity | CRC/RS 不匹配 | 记录 expected/actual；RS 修复后重新验 CRC |
| Record | object size/endbit/stream size 矛盾 | 限制在 object slice，跳过该对象 |
| Schema | dynamic class 不存在 | 保存 unknown/proxy object |
| Graph | dangling/duplicate/cyclic ownership | 报告；pointer cycle 可合法，ownership cycle 通常异常 |
| Semantic | 非有限坐标、负 count、非法 degree | 拒绝或隔离对象 |

### 26.2 资源限制

对 section size、page count、object count、EED size、字符串长度、嵌套 hatch/table/mesh arrays、SAB/blob total size 设置可配置上限。所有 `count × element_size`、`offset + size` 使用 checked arithmetic。解压器必须有输出上限，避免压缩炸弹。外部依赖路径和 VBA/OLE payload 只作为数据，不自动打开或执行。

### 26.3 Round-trip 等级

- **语义 round-trip**：几何和数据库关系等价，但 bytes 可变化。
- **结构 round-trip**：sections、对象字段和未知值保留，压缩/page 布局可变化。
- **byte-perfect round-trip**：未修改输入时完整 bytes 一致；只有保存所有 opaque 数据、padding、顺序和编码选择才可能实现。

规格符合性声明必须指出所达到的等级和覆盖版本。

<a id="dwg-ai-query-guide"></a>
## 27. AI 检索与推理约定

回答 DWG 问题时按以下顺序检索：

1. `dwg-version-matrix`：确认版本和“容器/对象编码”双重谱系；
2. 对应容器锚点：`dwg-r13-container`、`dwg-r2004-container`、`dwg-r2007-container`、`dwg-r2010-plus`；
3. `dwg-section-directory`：确认逻辑 section；
4. `dwg-object-record` 或 `dwg-entity-record`：确认 record 外壳和 stream 边界；
5. `dwg-type-field-index`：确认具体 type prescription；
6. 位编码锚点：如 `dwg-bitcode-bs`、`dwg-handle-reference`；
7. `dwg-validation`：检查错误处理和安全边界。

推理时必须显式写出版本前提。例如：“对 AC1024/R2010，容器按 R2004，object type 按 R2010，string/handle streams 按 R2007 模式。”不得从 DXF group code 直接推导 DWG 的物理字段位置；DXF code 只是语义映射。

<a id="dwg-known-ambiguities"></a>
## 28. 已知未知项与歧义

- 本规范来自逆向工程，不是 DWG 所有私有行为的完整定义。
- Header、AuxHeader、R13 unknown section 和空名称 section 含语义未知字段。
- R2004/R2007 的 encryption/security 只被部分描述。
- Dynamic/proxy classes 可在未来版本出现本文没有 prescription 的 fields。
- R2018 AuxHeader 增量字段在原文摘要与详细表的表述存在排版歧义。
- DataStorage search segment 的完整目的未知，但兼容写入不可简单省略。
- SPATIAL_INDEX 等内部索引 payload 只被有限描述。
- `CMC/ENC` 某些 RGB 后续类型在原文表述中不完全一致。
- 示例 hex dumps 是特定文件的实例，不构成默认值或唯一合法编码。

遇到这些情况，优先级为：对象/section 明确边界 > 版本条件 > CRC/size 一致性 > 已知字段 prescription > 样本推断。不得为了让后续字段“看起来合理”而忽略前面的长度或版本冲突。

<a id="dwg-original-chapter-map"></a>
## 29. ODA 原规范章节映射

| 原章 | 内容 | 本文章节 |
|---:|---|---|
| 1 | Introduction | 1–3 |
| 2 | Bit Codes and Data Definitions | 4–6 |
| 3 | R13–R2000 Organization（5.4.2 更名） | 7 |
| 4 | R2004 Organization | 8 |
| 5 | R2007 Organization | 9 |
| 6–8 | R2010/R2013/R2018 | 10 |
| 9 | Header Variables | 12 |
| 10 | Classes | 13 |
| 11–12 | Padding / empty-name section | 14.1 |
| 13–19 | Summary/Preview/VBA/AppInfo/FileDep/Rev/Security | 14.2–14.8 |
| 20 | AcDbObjects | 15–19 |
| 21–22 | ObjFreeSpace / Template | 14.9–14.10 |
| 23 | Handles/Object Map | 20 |
| 24 | DataStorage | 21 |
| 25 | Unknown Section | 14.12 |
| 26 | Second File Header (R13–R2000)（5.4.2 重写） | 7.4 |
| 27 | AuxHeader | 14.11 |
| 28 | EED | 22 |
| 29 | Proxy Entity Graphics | 23 |

<a id="dwg-reference-constants"></a>
## 附录 A：关键常量速查

| 常量 | 值/规则 |
|---|---|
| Classes beginning sentinel | `8D A1 C4 B8 C4 A9 F8 C5 C0 DC F4 5F E7 CF B6 8A` |
| R13–R15 second header sentinel | `D4 7B 21 CE 28 93 9F BF 53 24 40 09 12 3C AA 01` |
| R2004 physical header size | `0x100` |
| R2007 file header position/size | `0x80` / `0x400` |
| R2007 check-data position/size | `0x3D8` / `0x20` |
| CRC block alignment | 8 bytes |
| page/file alignment | 常见 `0x20` |
| R2007 system RS | `(255,239)` |
| R2007 data RS | `(255,251)` |
| Object Map max chunk | 2032 bytes |
| 常见 object-section page size | `0x7400` |
| R2004 data page header checksum | 0x18 = page header checksum（seed=data checksum），0x1C = data checksum（seed=0）（5.4.2 更正顺序） |
| DataStorage segment alignment | `0x40`；AutoCAD 常用 `0x80` |
| Direct DataStorage threshold | 约 `0x40000` bytes |
| DataStorage blob marker | `0xBB106BB1` |
| ACIS prefix | `ACIS BinaryFile` |

<a id="dwg-glossary"></a>
## 附录 B：术语表

| 术语 | 定义 |
|---|---|
| ACI | AutoCAD Color Index |
| ACIS/SAT/SAB | 几何内核及其文本/二进制实体表示 |
| ASM | Autodesk Shape Manager，ACIS 衍生内核 |
| classdxfname | Classes table 中用于识别动态 DWG 类型的 DXF 名称 |
| endbit | data/string 区结束或 handle stream 开始的 bit offset |
| logical section | 由多个物理 pages 重建的连续命名数据流 |
| maintenance version | 同一 AC 文件版本内的格式修订号 |
| owner | 决定对象所属和生命周期的 handle 关系 |
| pointer | 不改变所有权的对象引用 |
| reactor | 对对象事件/依赖关系的持久引用 |
| proxy | 当前实现不认识其 class prescription 时的保留对象/实体 |
| zero page | 逻辑上全零、可不物理写出的 page |

<a id="dwg-source-note"></a>
## 附录 C：来源说明

来源文档：Open Design Alliance, *Open Design Specification for .dwg files*, Version 5.4.1, 279 pages。本文重组其技术事实并加入显式版本矩阵、边界规则和 AI 检索锚点；没有结合任何具体代码实现。若本文与原 PDF 的逐字段 prescription 冲突，以原 PDF 对相应版本的字段顺序为回查依据，并记录冲突以修订本文。

<a id="dwg-header-ordered-prescription"></a>
## 附录 D：Header Variables 有序字段处方

本附录按原规范 pp.72–83 的逻辑读取顺序列出。逗号分隔字段保持从左到右顺序；同一行编码适用于该行全部字段。`H*` 表示该 handle 在 R2007+ 可能物理位于 handle stream，但逻辑顺序仍在此处。变量组不允许排序或按名称查表后随意读取。

### D.1 包装、前导和基础 flags

| 顺序 | 版本 | 编码 | 字段 |
|---:|---|---|---|
| 1 | common | `SN` | begin=`CF 7B 1F 23 FD DE 38 A9 5F 7C 68 B8 4E 6D 33 5F` |
| 2 | common | `RL` | section byte size |
| 3 | R2010/R2013 maint>3；R2018+ | `RL` | **未知**，可能为 64-bit size 高半部 |
| 4 | R2007 only | `RL` | section data bit size |
| 5 | R2013+ | `BLL` | REQUIREDVERSIONS，默认 0 |
| 6 | common | `BD×4` | **未知**；典型默认 `412148564080.0, 1.0, 1.0, 1.0` |
| 7 | common | `TV×4` | **未知文本**，默认空 |
| 8 | common | `BL×2` | **未知**，典型默认 24、0 |
| 9 | R13–R14 | `BS` | **未知**，默认 0 |
| 10 | pre-R2004 | `H*` | current viewport entity header |
| 11 | common | `B` | DIMASO, DIMSHO |
| 12 | R13–R14 | `B` | DIMSAV（未公开语义） |
| 13 | common | `B` | PLINEGEN, ORTHOMODE, REGENMODE, FILLMODE, QTEXTMODE, PSLTSCALE, LIMCHECK |
| 14 | R13–R14 | `B` | BLIPMODE |
| 15 | R2004+ | `B` | **未知 flag** |
| 16 | common | `B` | USRTIMER, SKPOLY, ANGDIR, SPLFRAME |
| 17 | R13–R14 | `B` | ATTREQ, ATTDIA |
| 18 | common | `B` | MIRRTEXT, WORLDVIEW |
| 19 | R13–R14 | `B` | WIREFRAME（未公开语义） |
| 20 | common | `B` | TILEMODE, PLIMCHECK, VISRETAIN |
| 21 | R13–R14 | `B` | DELOBJ |
| 22 | common | `B` | DISPSILH, PELLIPSE |
| 23 | common | `BS` | PROXYGRAPHICS |
| 24 | R13–R14 | `BS` | DRAGMODE |

### D.2 整数和实数系统变量

| 顺序 | 版本 | 编码 | 字段（严格有序） |
|---:|---|---|---|
| 25 | common | `BS` | TREEDEPTH, LUNITS, LUPREC, AUNITS, AUPREC |
| 26 | R13–R14 | `BS` | OSMODE |
| 27 | common | `BS` | ATTMODE |
| 28 | R13–R14 | `BS` | COORDS |
| 29 | common | `BS` | PDMODE |
| 30 | R13–R14 | `BS` | PICKSTYLE |
| 31 | R2004+ | `BL×3` | **未知 long 1–3** |
| 32 | common | `BS` | USERI1, USERI2, USERI3, USERI4, USERI5 |
| 33 | common | `BS` | SPLINESEGS, SURFU, SURFV, SURFTYPE, SURFTAB1, SURFTAB2, SPLINETYPE |
| 34 | common | `BS` | SHADEDGE, SHADEDIF, UNITMODE, MAXACTVP, ISOLINES, CMLJUST, TEXTQLTY |
| 35 | common | `BD` | LTSCALE, TEXTSIZE, TRACEWID, SKETCHINC, FILLETRAD, THICKNESS, ANGBASE, PDSIZE, PLINEWID |
| 36 | common | `BD` | USERR1, USERR2, USERR3, USERR4, USERR5 |
| 37 | common | `BD` | CHAMFERA, CHAMFERB, CHAMFERC, CHAMFERD, FACETRES, CMLSCALE, CELTSCALE |
| 38 | R13–R2004 | `TV` | MENUNAME |
| 39 | common | `BL×2` | TDCREATE Julian day、milliseconds into day |
| 40 | common | `BL×2` | TDUPDATE Julian day、milliseconds into day |
| 41 | R2004+ | `BL×3` | **未知时间附近 long 1–3** |
| 42 | common | `BL×2` | TDINDWG days、milliseconds into day |
| 43 | common | `BL×2` | TDUSRTIMER days、milliseconds into day |
| 44 | common | `CMC` | CECOLOR |

### D.3 当前对象、Paper Space 与 Model Space

| 顺序 | 版本 | 编码 | 字段（严格有序） |
|---:|---|---|---|
| 45 | common | `H` | HANDSEED；即使 R2007+ 仍在 data stream |
| 46 | common | `H*` | CLAYER, TEXTSTYLE, CELTYPE |
| 47 | R2007+ | `H*` | CMATERIAL |
| 48 | common | `H*` | DIMSTYLE, CMLSTYLE |
| 49 | R2000+ | `BD` | PSVPSCALE |
| 50 | common | `3BD,3BD,3BD,2RD,2RD,BD` | PSPACE INSBASE, EXTMIN, EXTMAX, LIMMIN, LIMMAX, ELEVATION |
| 51 | common | `3BD×3,H*` | PSPACE UCSORG, UCSXDIR, UCSYDIR, UCSNAME |
| 52 | R2000+ | `H*,BS,H*` | PUCSORTHOREF, PUCSORTHOVIEW, PUCSBASE |
| 53 | R2000+ | `3BD×6` | PUCSORGTOP, PUCSORGBOTTOM, PUCSORGLEFT, PUCSORGRIGHT, PUCSORGFRONT, PUCSORGBACK |
| 54 | common | `3BD,3BD,3BD,2RD,2RD,BD` | MSPACE INSBASE, EXTMIN, EXTMAX, LIMMIN, LIMMAX, ELEVATION |
| 55 | common | `3BD×3,H*` | MSPACE UCSORG, UCSXDIR, UCSYDIR, UCSNAME |
| 56 | R2000+ | `H*,BS,H*` | UCSORTHOREF, UCSORTHOVIEW, UCSBASE |
| 57 | R2000+ | `3BD×6` | UCSORGTOP, UCSORGBOTTOM, UCSORGLEFT, UCSORGRIGHT, UCSORGFRONT, UCSORGBACK |

### D.4 Dimension variables

| 顺序 | 版本 | 编码 | 字段（严格有序） |
|---:|---|---|---|
| 58 | common | `TV×2` | DIMPOST, DIMAPOST |
| 59 | R13–R14 | `B` | DIMTOL, DIMLIM, DIMTIH, DIMTOH, DIMSE1, DIMSE2, DIMALT, DIMTOFL, DIMSAH, DIMTIX, DIMSOXD |
| 60 | R13–R14 | `RC/B/BS/H*` | DIMALTD, DIMZIN, DIMSD1, DIMSD2, DIMTOLJ, DIMJUST, DIMFIT, DIMUPT, DIMTZIN, DIMALTZ, DIMALTTZ, DIMTAD, DIMUNIT, DIMAUNIT, DIMDEC, DIMTDEC, DIMALTU, DIMALTTD, DIMTXSTY |
| 61 | common | `BD` | DIMSCALE, DIMASZ, DIMEXO, DIMDLI, DIMEXE, DIMRND, DIMDLE, DIMTP, DIMTM |
| 62 | R2007+ | `BD,BD,BS,CMC` | DIMFXL, DIMJOGANG, DIMTFILL, DIMTFILLCLR |
| 63 | R2000+ | `B` | DIMTOL, DIMLIM, DIMTIH, DIMTOH, DIMSE1, DIMSE2 |
| 64 | R2000+ | `BS` | DIMTAD, DIMZIN, DIMAZIN |
| 65 | R2007+ | `BS` | DIMARCSYM |
| 66 | common | `BD` | DIMTXT, DIMCEN, DIMTSZ, DIMALTF, DIMLFAC, DIMTVP, DIMTFAC, DIMGAP |
| 67 | R13–R14 | `T×5` | DIMPOST, DIMAPOST, DIMBLK, DIMBLK1, DIMBLK2（旧式重复位置） |
| 68 | R2000+ | `BD,B,BS,B,B,B,B,B` | DIMALTRND, DIMALT, DIMALTD, DIMTOFL, DIMSAH, DIMTIX, DIMSOXD |
| 69 | common | `CMC×3` | DIMCLRD, DIMCLRE, DIMCLRT |
| 70 | R2000+ | `BS` | DIMADEC, DIMDEC, DIMTDEC, DIMALTU, DIMALTTD, DIMAUNIT, DIMFRAC, DIMLUNIT, DIMDSEP, DIMTMOVE, DIMJUST |
| 71 | R2000+ | `B,B,BS,BS,BS,B,BS` | DIMSD1, DIMSD2, DIMTOLJ, DIMTZIN, DIMALTZ, DIMALTTZ, DIMUPT, DIMATFIT |
| 72 | R2007+ | `B` | DIMFXLON |
| 73 | R2010+ | `B,BD,T,BD,T` | DIMTXTDIRECTION, DIMALTMZF, DIMALTMZS, DIMMZF, DIMMZS |
| 74 | R2000+ | `H*` | DIMTXSTY, DIMLDRBLK, DIMBLK, DIMBLK1, DIMBLK2 |
| 75 | R2007+ | `H*` | DIMLTYPE, DIMLTEX1, DIMLTEX2 |
| 76 | R2000+ | `BS×2` | DIMLWD, DIMLWE |

### D.5 Control roots、dictionaries 和现代变量

| 顺序 | 版本 | 编码 | 字段（严格有序） |
|---:|---|---|---|
| 77 | common | `H*` | BLOCK, LAYER, STYLE, LINETYPE, VIEW, UCS, VPORT, APPID, DIMSTYLE control objects |
| 78 | R13–R15 | `H*` | VIEWPORT ENTITY HEADER control |
| 79 | common | `H*` | ACAD_GROUP dictionary, ACAD_MLINESTYLE dictionary, NAMED OBJECTS dictionary |
| 80 | R2000+ | `BS,BS,TV,TV` | TSTACKALIGN, TSTACKSIZE, HYPERLINKBASE, STYLESHEET |
| 81 | R2000+ | `H*` | LAYOUTS, PLOTSETTINGS, PLOTSTYLES dictionaries |
| 82 | R2004+ | `H*` | MATERIALS, COLORS dictionaries |
| 83 | R2007+ | `H*` | VISUALSTYLE dictionary |
| 84 | R2013+ | `H*` | **未知 hard pointer** |
| 85 | R2000+ | `BL` | packed flags：CELWEIGHT、ENDCAPS、JOINSTYLE、LWDISPLAY、XEDIT、EXTNAMES、PSTYLEMODE、OLESTARTUP |
| 86 | R2000+ | `BS,BS,[H*],TV,TV` | INSUNITS, CEPSNTYPE, CPSNID if type=3, FINGERPRINTGUID, VERSIONGUID |
| 87 | R2004+ | `RC` | SORTENTS, INDEXCTL, HIDETEXT, XCLIPFRAME, DIMASSOC, HALOGAP |
| 88 | R2004+ | `BS,BS,RC,RC,TV` | OBSCUREDCOLOR, INTERSECTIONCOLOR, OBSCUREDLTYPE, INTERSECTIONDISPLAY, PROJECTNAME |
| 89 | common | `H*` | PAPER_SPACE block record, MODEL_SPACE block record, BYLAYER/BYBLOCK/CONTINUOUS linetypes |
| 90 | R2007+ | `B,BL,BL,BD` | CAMERADISPLAY，三个字段中后 3 个为**未知** |
| 91 | R2007+ | `BD` | STEPSPERSEC, STEPSIZE, 3DDWFPREC, LENSLENGTH, CAMERAHEIGHT |
| 92 | R2007+ | `RC` | SOLIDHIST, SHOWHIST |
| 93 | R2007+ | `BD` | PSOLWIDTH, PSOLHEIGHT, LOFTANG1, LOFTANG2, LOFTMAG1, LOFTMAG2 |
| 94 | R2007+ | `BS,RC` | LOFTPARAM, LOFTNORMALS |
| 95 | R2007+ | `BD,BD,BD,BL` | LATITUDE, LONGITUDE, NORTHDIRECTION, TIMEZONE |
| 96 | R2007+ | `RC` | LIGHTGLYPHDISPLAY, TILEMODELIGHTSYNCH, DWFFRAME, DGNFRAME |
| 97 | R2007+ | `B,CMC,H*,H*,H*,RC,BD` | **未知 flag**、INTERFERECOLOR、INTERFEREOBJVS、INTERFEREVPVS、DRAGVS、CSHADOW、**未知 BD** |
| 98 | R14+ 特定 header type | `BS×4` | **未知 shorts**；规范称似乎非必需，但必须按实际 section prescription 消费 |
| 99 | common | padding + `RS` | 对齐到字节；CRC seed=`0xC0C1`，覆盖 sentinel 后的数据 |
| 100 | common | `SN` | end=`30 84 E0 DC 02 21 C7 56 A0 83 97 47 B1 92 CC A0` |

Header 新建写入时应使用目标版本的精确子序列；读后写时保留所有未知值。不能用“所有已知变量的集合”替代上述序列，因为不存在的旧版本字段会导致整条 bit stream 错位。
