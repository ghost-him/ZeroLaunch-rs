# ZeroLaunch-rs

该软件还处于早期阶段，目前已经成熟 CPP 版本的地址如下：[github](https://github.com/ghost-him/ZeroLaunch-CPP)。

## 介绍

ZeroLaunch-rs 是一个使用 Rust + Tauri + Vite + Vue.js + TypeScript 构建的运行在 windows 环境下的用于快速启动应用程序的软件。

软件特点：

- 内存占用极低
- 纯本地运行，不联网
- 高度自定义的界面
- 开源，永久免费
- 优秀的搜索算法

该软件因个人需要而开发，因此会长期维护！

## todo

- [ ] 构建使用界面，设置界面
- [x] 使用 `Alt+Space` 唤出搜索栏
- [x] 使用 `Ctrl+j/k` 进行选项的上下移动
- [x] 使用 `Up/Down` 进行选项的上下移动
- [ ] 使用 `Alt+数字键` 进行搜索内容的切换
- [ ] 使用 `ESC` 完成清屏，隐藏搜索栏
- [ ] 根据输入的字符串进行程序的匹配


### 优化方向

1. 优化前端与后端的数据传输：使用 protocol 代替默认的数据传输
2. 使用ac自动机优化搜索算法（待讨论）
