## Examples

构建并运行将位于 `examples/` 下的示例代码，将输出保存到 `website/static/examples/` 下。

每一个示例程序应当为一个文件夹而非单文件，目录结构如下：

```
examples/xxx/
├── main.rs
└── README.md (Optional)
```

运行后直接将 `output/xxx` 目录复制到 `website/static/examples/xxx` 目录，并在 `website/data/` 下生成一个 `xxx.toml`，包含如下内容：

```toml
name = "xxx"
code = "<markdown code block of main.rs>"
output_type = "image" # or "video"
output_path = "examples/xxx/output.png" # or "examples/xxx/output.mp4"

在 zola 的 markdown 中使用 `!example-xxx` 来插入示例。