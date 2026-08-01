# Kimi Planbar Tray

[English](README.md)

一个轻量的 Windows 托盘程序，让 [Kimi Code](https://www.kimi.com/code/) 套餐额度随手可查——5 小时窗口和每周用量，带重置倒计时，就在系统托盘里。

| 月之亮面 | 月之暗面 |
|---|---|
| ![亮面](docs/screenshot-light.png) | ![暗面](docs/screenshot-dark.png) |

## 功能

- **托盘常驻**——左键图标弹出悬浮窗（Windows 原生滑出动画），失焦自动隐藏；右键菜单：打开 / 刷新 / 设置 / 退出
- **额度一目了然**——5 小时与本周用量双卡片，进度条 + 重置倒计时；数据与 CLI 的 `/usage` 同源
- **亮暗双主题**——月之亮面 / 月之暗面，可跟随 Windows 系统主题实时切换（也可在设置里固定）；强调色 `#1A88FF`
- **抗抖动刷新**——按设定间隔自动刷新（1/5/10/30 分钟）；失败时保留上一次成功的数据，30 秒后快速重试
- **CLI 版本检测**——显示本机 `kimi --version`；[kimi-code Releases](https://github.com/MoonshotAI/kimi-code/releases) 有新版时出现橙色徽章（点击版本行直达发布页）；GitHub 不可达时静默降级
- **绿色免 UAC**——单 exe，仅操作用户域（自启走 HKCU，不碰 HKLM 和 Program Files）；在 exe 旁放一个空的 `portable.dat` 即切换为配置随身携带的便携模式
- **占用小**——单文件版 ~195 KB，运行内存 ~80 MB，除刷新定时器外无任何后台轮询

## 下载

从 [Releases](../../releases) 获取最新 exe：

| 版本 | 体积 | 前提 |
|---|---|---|
| `KimiPlanbarTray.exe` | ~195 KB | 已安装 [.NET 8 桌面运行时](https://dotnet.microsoft.com/download/dotnet/8.0) |
| `KimiPlanbarTray-selfcontained.exe` | ~65 MB | 无——运行时已打包在内 |

> exe 未做代码签名，首次运行 Windows SmartScreen 可能提示"已保护你的电脑"——点"更多信息 → 仍要运行"即可，这是未签名个人作品的正常提示。

## 使用前提

- Windows 10 / 11
- 已安装并登录 [Kimi Code](https://www.kimi.com/code/) CLI，且为 **Kimi For Coding** 套餐用户（程序从 `~/.kimi-code/credentials/kimi-code.json` 读取 CLI 的本地 OAuth token，兜底读 `~/.kimi-code/config.toml` 里的明文 api_key）
- 能访问 `api.kimi.com`

凭证不会被存储或发送到官方 `api.kimi.com/coding/v1/usages` 接口以外的任何地方。

## 使用说明

- **左键托盘图标**——显示 / 隐藏悬浮窗
- **右键托盘图标**——菜单：打开 / 刷新 / 设置 / 退出
- **悬停托盘图标**——速览 `5h X% · week Y%`
- **设置**——主题（跟随系统 / 月之亮面 / 月之暗面）、刷新间隔、开机自启
- **版本行**——点击打开 Releases 页面

## 从源码构建

需要 .NET 8 SDK（Windows）：

```bash
dotnet publish -c Release -r win-x64 --self-contained false -p:PublishSingleFile=true -o publish
# 自包含版：
dotnet publish -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -p:EnableCompressionInSingleFile=true -o publish-sc
```

无头自检（适合 CI 或改动后验证）：

```bash
KimiPlanbarTray.exe --test-fetch   # 拉取一次额度，打印 JSON 后退出
KimiPlanbarTray.exe --test-ui      # 构造两个窗口，验证资源与 XAML 加载
```

## 技术说明

- .NET 8 / WPF，零第三方 NuGet 依赖
- 额度逻辑移植自 [kimi-planbar](https://github.com/baigong-ai/kimi-planbar)（MIT）——token 来源、接口与缓存/重试策略一致
- 托盘图标为内嵌的官方 Kimi Code logo（PNG 压缩 ICO）；logo 版权归 **Moonshot AI** 所有——本项目为非官方社区工具，与 Moonshot AI 无隶属关系

## License

[MIT](LICENSE) © 2026 Shawn Qi (shawn-0106t)，部分内容 © baigong-ai (kimi-planbar)
