# FFI Phase 4a 字节读写设计规格

## 1. 目标与范围

P4a 为现有 FFI 增加 DWG 内存字节流读写能力，使 C 调用者可以执行
`bytes -> DwgData -> bytes`，并明确跨 CRT 分配与释放规则。本阶段只处理 DWG
字节流，不实现 Dictionary、XRECORD、EED、ACIS 或通知回调。

## 2. ABI

```c
int dwg_read_bytes(const uint8_t *input, size_t input_len, DwgData *data);
int dwg_write_bytes(const DwgData *data, uint8_t **output, size_t *output_len);
void dwg_free_buffer(void *buffer);
```

- `dwg_read_bytes` 要求 `input`、`data` 非 NULL 且 `input_len > 0`；无效参数返回
  `DWG_EINVAL`。成功后替换文档，失败时保留原文档内容。
- `dwg_write_bytes` 要求 `data`、`output`、`output_len` 非 NULL。进入写入前将
  `*output = NULL`、`*output_len = 0`；失败时保持该状态并返回 `DWG_EWRITE`。
- 成功时 `output` 指向由 FFI 分配的完整 DWG 字节序列，`output_len` 为字节数；
  调用者必须且只能使用 `dwg_free_buffer` 释放该指针，不得使用 `free`。
- `dwg_free_buffer(NULL)` 是安全空操作。非 NULL 指针必须来自本 FFI 的成功写入，
  不允许传入偏移指针、栈地址或重复释放的指针。

## 3. 所有权与分配

Rust 的 `Vec<u8>` 不能直接转换成只含裸数据指针的 `Box<[u8]>` 后再由无长度
参数的释放函数恢复。实现使用 FFI 私有分配头保存长度：分配块为
`[BufferHeader][payload bytes]`，返回 payload 地址；释放时从 payload 回退到头部，
读取长度并以同一 Rust allocator 释放整个块。C ABI 不暴露头部布局。

所有溢出、分配失败和空输出都必须在写入前或分配阶段处理；不得返回需要 C
运行库释放的内存。

## 4. Rust 实现边界

- `src/ffi/io.rs` 增加字节入口，并复用 `DwgReader::from_stream` 与
  `DwgWriter::write_to_vec`。
- `src/ffi/types.rs` 保持 opaque 类型和现有错误码不变。
- 成功调用清空线程本地错误；失败调用设置与返回值一致的错误码和消息。
- `dwg_read_bytes` 先完成解析，再通过现有 `replace_document` 替换 arena，避免
  部分解析结果破坏调用者已有文档。

## 5. 验收测试

1. Rust ABI 测试覆盖 NULL 参数、空输入、写入失败、输出归零、成功清错及
   `dwg_free_buffer` 释放路径。
2. 代表性 C smoke 测试真实编译、链接并运行：创建文档，写出字节，读取字节，
   检查版本和对象数量，最后调用 FFI 释放输出。
3. 使用已有 DWG fixture 或 Rust writer 生成的数据验证 write/read round-trip；
   不比较内部指针地址，只比较可观察版本、实体数量和实体字段。
4. 运行 `cargo test --release --features ffi`，并保持本阶段不修改无关 codec。

## 6. 非目标

本阶段不提供 DXF 字节入口，不改变文件读写 API，不生成公共头文件，不引入
`cbindgen`，也不实现 P4b/P4c 内容。
