# examples/agents

本目录存放由 agent 以 one-shot 方式编写的 ranim example。

目录规则、README 要求与 ranim-cli 迭代流程见 [AGENTS.md](AGENTS.md)。

## 索引

| Example | 一句话说明 | 模型 | 生成日期 | 状态 |
|---|---|---|---|---|
| [double_pendulum](double_pendulum/) | 三个初始角仅差 0.001 rad 的双摆从重叠到彻底分离，演示混沌的初值敏感性 | Kimi（Moonshot AI，具体版本未确认） | 2026-08-18 | 已渲染并视觉检查 |
| [rubiks_cube](rubiks_cube/) | 三阶魔方「12 步打乱 → 逆序求解」全过程，3D 魔方与平面展开图同步更新 | Kimi K3（kimi-code/k3） | 2026-08-18 | 已渲染并视觉检查 |
