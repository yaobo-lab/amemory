你参考 examples\clippi 这个项目的UI

为当前项目src 添加一个GUI 

clippi 使用的是 
gpui = { version = "0.2.2", default-features = false }
gpui-component = "0.5"
gpui_transitions = "0.1.5"

作为GUI 框架，但当前项目不要这个

项目选型 ：https://github.com/longbridge/nsh 作为GUI框架

你只需要参考 clippi 的UI风格，还有窗口大小，按扭等全部一比一的复刻。 然后将本项目的一些数据显，参数设置等接上UI


ui 相关的代码放到 src\ui 目录中 逻辑放到对应的 src\core、services、platform、state . 如果用到音频，图片等资源放到 assets 中 
要实现前端与 逻辑代码分离。


你先出个实施计划。到docs 目录。我检查过无问题再执行