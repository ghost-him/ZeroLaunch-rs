# ZeroLaunch-rs

该软件还处于早期阶段，目前已经成熟 CPP 版本的地址如下：[github](https://github.com/ghost-him/ZeroLaunch-CPP)。

## 介绍

ZeroLaunch-rs 是一个使用 Rust + Tauri + Vite + Vue.js + TypeScript 构建的运行在 windows 环境下的用于快速启动应用程序的软件。

软件特点：

- 纯本地运行，不联网
- 高度自定义的界面
- 开源，永久免费
- 优秀的搜索算法

该软件因个人需要而开发，因此会长期维护！

## 软件截图

程序运行主界面

![](https://raw.githubusercontent.com/ghost-him/ZeroLaunch-rs/refs/heads/main/asset/%E4%B8%BB%E7%95%8C%E9%9D%A2.png)

程序搜索界面

![](https://raw.githubusercontent.com/ghost-him/ZeroLaunch-rs/refs/heads/main/asset/%E7%B2%BE%E5%87%86%E5%8C%B9%E9%85%8D.png)

设置界面

![](https://raw.githubusercontent.com/ghost-him/ZeroLaunch-rs/refs/heads/main/asset/%E8%AE%BE%E7%BD%AE%E7%95%8C%E9%9D%A2.png)

更多内容：[点击跳转](https://github.com/ghost-him/ZeroLaunch-rs/blob/main/asset/picture.md)


## 快速上手

* 呼出软件搜索栏：`Alt + Space`
* 搜索程序：在搜索栏中输入目标程序的名字
* 选择程序：键盘的方向键 `上` 与方向键 `下` 或使用 `Ctrl + k` 与 `Ctrl + j` （vim的操作模式）
* 启动程序：使用鼠标点击目标程序 或 按下 `enter` 键
  * 使用管理员权限启动程序：执行上述操作时按下 `ctrl` 键
* 清除搜索栏的文字：按下 `ESC` 
* 隐藏搜索栏：点击搜索栏以外的区域 或 在搜索栏没有文字时按下 `ESC`

## 构建方法

1. 下载项目的源代码，解压
2. 进入项目的根目录
3. `yarn install`
4. debug模式：`yarn tauri dev`；release：`yarn tauri build`。
5. 打包好的软件会在`./src-tauri/target`目录下。

## 注意事项

* 该软件的数据与日志存放在: `[用户根目录]\AppData\Roaming\ZeroLaunch-rs` 文件夹下。
* 当用户输入的长度小于3时，搜索算法不能准确判断用户的输入。

## 已有bug

* 使用rime输入法时，会出现卡死的情况，目前无法解决

## todo

- [x] 构建使用界面，设置界面
- [x] 使用 `Alt+Space` 唤出搜索栏
- [x] 使用 `Ctrl+j/k` 进行选项的上下移动
- [x] 使用 `Up/Down` 进行选项的上下移动
- [ ] 使用 `Alt+数字键` 进行搜索内容的切换
- [x] 使用 `ESC` 完成清屏，隐藏搜索栏
- [x] 根据输入的字符串进行程序的匹配
- [x] 添加自动启动的功能（支持静默启动）
- [x] 添加自定义搜索网页的功能（当确定是网页时，使用系统默认的浏览器打开）
- [x] 启动程序时，默认使用用户权限，按下ctrl再启动时，使用管理员权限
- [x] 自定义搜索栏选项的颜色
- [x] 自定义搜索栏选项的图片

