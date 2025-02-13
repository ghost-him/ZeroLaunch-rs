# ZeroLaunch-rs

## 介绍

ZeroLaunch-rs 是一个使用 Rust + Tauri + Vite + Vue.js + TypeScript 构建的运行在 windows 环境下的用于快速启动应用程序的软件。

软件特点：

- 纯本地运行，不联网。
- 高度自定义的界面。
- 开源，永久免费。
- 优秀的搜索算法，支持全称，模糊，拼音搜索（拼音搜索也支持模糊搜索）。

该软件因个人需要而开发，因此会长期维护！

## 软件截图

程序运行主界面

![](https://raw.githubusercontent.com/ghost-him/ZeroLaunch-rs/refs/heads/main/asset/%E4%B8%BB%E7%95%8C%E9%9D%A2.png)

程序搜索界面

![](https://raw.githubusercontent.com/ghost-him/ZeroLaunch-rs/refs/heads/main/asset/%E7%B2%BE%E5%87%86%E5%8C%B9%E9%85%8D.png)

**背景图片可在设置界面更换**

设置界面

![](https://raw.githubusercontent.com/ghost-him/ZeroLaunch-rs/refs/heads/main/asset/%E8%AE%BE%E7%BD%AE%E7%95%8C%E9%9D%A2.png)

更多详细的屏蔽内容：[点击跳转](https://github.com/ghost-him/ZeroLaunch-rs/blob/main/asset/picture.md)

## 快速上手

### 基本操作

* 呼出软件搜索栏：`Alt + Space`
* 搜索程序：在搜索栏中输入目标程序的名字
* 选择程序：键盘的方向键 `上` 与方向键 `下` 或使用 `Ctrl + k` 与 `Ctrl + j` （vim的操作模式）
* 启动程序：使用鼠标点击目标程序 或 按下 `enter` 键
  * 使用管理员权限启动程序：执行上述操作时按下 `ctrl` 键
* 清除搜索栏的文字：按下 `ESC` 
* 隐藏搜索栏：点击搜索栏以外的区域 或 在搜索栏没有文字时按下 `ESC`

### 使用同步网盘同步配置文件

当前版本已经支持自定义配置文件的保存地址了，所以可以使用第三方的同步网盘（比如onedrive，坚果云，百度网盘等）同步配置文件，从而实现多端数据（包含配置信息，应用打开次数统计，背景图片等）共享。

注意，设置了文件夹以后会直接将配置文件创建在目标文件夹下，一共会创建两个文件，一个用于储存程序的配置信息，一个用于储存背景图片。

设置方法：进入程序的设置界面，找到 “其它设置” 标签页，点击 “选择目标路径” 按钮，然后选择与同步网盘关联的同步文件夹即可。推荐在文件夹下新建一个 `ZeroLaunch` 子文件夹，然后选择这个子文件夹。

### 自定义搜索的路径与不搜索的路径

* b对于自定义搜索路径

自定义搜索路径的文件夹搜索深度为 5 层：程序会递归搜索给定根目录下的所有文件，并遍历其下最多 5 层子文件夹 中的内容。具体规则如下：

根目录（第 0 层）及其直接子文件夹（第 1 层）会被完全索引。
每个子文件夹的递归深度逐层递减，直到达到第 5 层后停止（共支持根目录 + 5 层子目录）。
程序仅索引 .exe、.url、.lnk 三类文件，并自动过滤名称含屏蔽关键字的文件。
例如以下文件夹结构：

```
C:\users\ghost\desktop\root_folder
└─folder_1
    └─folder_2
        └─folder_3
            └─folder_4
                └─folder_5
                    └─folder_6

```

若自定义路径为 C:\users\ghost\desktop\root_folder，程序会索引：

root_folder（第 0 层）
folder_1（第 1 层）
folder_2（第 2 层）
folder_3（第 3 层）
folder_4（第 4 层）
folder_5（第 5 层）
但不会搜索 folder_6（第 6 层，超出深度限制）。

* 对于不搜索路径

如果不想要搜索某一文件夹，则可以将该文件夹的路径写入。不搜索路径要求：搜索路径的前缀与不搜索路径完全匹配。

以上例为例：如果写入了 `C:\users\ghost\desktop`，则不会遍历该路径，而写入了 `C:\users\ghost\desktop\root folder\folder 1` 时，只会遍历 `root folder` 下的所有文件与除了 `folder 1` 之外的所有的子文件夹下的所有的文件。

### 屏蔽字

屏蔽字的作用为：当你有一些程序不想被索引，比如xxx程序的卸载程序，还有xxx程序的帮助文档。你可以将其共性的词，比如`卸载(uninstall)`与`帮助(help)`添加到屏蔽字中，这样程序在索引程序时，在检测到这个关键字以后就会跳过，从而减少算法的运行时间。

### 关键字过滤器

通过使用关键字过滤器，你可以自定义目标应用程序的出现的权重。

每一个应用程序都有一个值，叫 `compatibility` ，这个值的意思为：当前程序与用户搜索的匹配度。而程序中显示出来的程序则是所有程序中 `compatibility` 最大的几个。

而关键字过滤器可以自定义包含关键字的程序的 `compatibility`。

关键字过滤器的计算方式如下：当一个程序的名字中出现了一个关键字，则其 `compatibility` 会加上这个关键字对应的值（可正可负）。如果出现了多个关键字，则会累加。

案例：可以看到程序中有默认关键字过滤器： `卸载` 与 `uninstall`，对应的值都为 `-5000`。因此，拥有这个关键字的程序的 `compatibility` 都会减 `5000`，最终会使得这些程序永远不会出现在搜索结果框中。

推荐的范围：`[-10.0, 10.0]`。

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

# 使用到了开源项目

* [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua)：程序使用该项目提供的字典（经过处理）来完成中文转拼音。

* [Bootstrap](https://icons.bootcss.com/)：提供了免费使用的高质量图标。

* [LaunchyQt](https://github.com/samsonwang/LaunchyQt)：提供了索引 UWP 应用程序的代码。
