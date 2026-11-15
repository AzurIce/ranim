# KDL 表达草案

下面是用于讨论的场景表示示例，类型标记和字段结构仅作示意，**不是已经支持解析或
导出的格式**。当前定义树保留初始快照、配方与播放配置；类型注册和 `Any` downcast
不提供任意输入、配方或闭包的序列化 codec。

完整场景格式及其编码/解码仍需单独定义。`inspect tree` 的 JSON 和 WASM timeline
属性快照也不是这种场景格式，详见[动画系统：检查与持久化的边界](../understand/core/anim.md)。

```kdl
("anim:demo@0.1.0:Sequence")root {
    duration 4.0

    children {
        ("anim:demo@0.1.0:FadeIn")child {
            // duration #null // 不覆盖内容原始时长（FadeIn 为 1 秒）
            // rate linear
            // enabled #true

            ("item:demo@0.1.0:Circle")target {
                radius 0.5
            }
        }

        ("anim:demo@0.1.0:Rotate")child {
            duration 2.0  // 公共字段
            enabled #true

            angle 1.57       // Rotate 的字段
            axis 0.0 0.0 1.0
            ("item:demo@0.1.0:Circle")target {
                radius 0.5
            }
        }
    }
}
```
