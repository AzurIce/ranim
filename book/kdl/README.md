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
