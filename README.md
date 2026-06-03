# 泡泡大作战 (Bubble Game)

基于 Rust + WASM 的跨端浏览器泡泡吞噬游戏。游戏逻辑与 Canvas 渲染均在客户端 WASM 中运行，部署到 nginx 后仅做静态文件分发，不占用服务器计算资源。

## 玩法

- 操控粉色泡泡在有限地图中移动，吃掉单位泡泡（小能量点）或其它更小的泡泡来增大自己
- 初始阶段只能吃单位泡泡；吃掉第一个非单位泡泡后可吞噬更小的 AI 泡泡
- 当 2/3 面积被更大泡泡遮盖时会立即死亡，并掉落 2/3 能量供其它泡泡吸收
- 按住空格（电脑）或右下角按钮（手机）可排空部分能量换取加速
- 当泡泡直径达到屏幕短边的 2/3 时胜利

### 电脑端操作

- 左键点击：开始/暂停鼠标方向控制
- 鼠标移动：控制方向
- 空格：排空能量
- R：重新开始

### 手机端操作（横屏）

- 左下虚拟摇杆：控制方向
- 右下「排空」按钮：排出能量

## 构建

需要安装 [Rust](https://rustup.rs/) 和 [wasm-pack](https://rustwasm.github.io/wasm-pack/):

```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
rustup target add wasm32-unknown-unknown
```

编译：

```bash
chmod +x build.sh
./build.sh
```

产物位于 `www/pkg/`，与 `www/index.html` 等一起构成完整站点。

## 本地预览

```bash
# 任选一种静态服务器
npx serve www
# 或
python3 -m http.server 8080 --directory www
```

浏览器访问 `http://localhost:8080`（Safari 建议使用 HTTPS 或 localhost）。

## nginx 部署

将 `www/` 目录上传到服务器，例如 `/var/www/games/bubble/`：

```nginx
location /bubble/ {
    alias /var/www/games/bubble/;
    index index.html;
    try_files $uri $uri/ /bubble/index.html;

    types {
        application/wasm wasm;
    }
}
```

客户端首次访问会下载 HTML、JS、WASM；之后所有游戏逻辑在浏览器本地运行，服务器不再消耗 CPU。

## 项目结构

```
buble_game/
├── src/           # Rust 游戏核心
├── www/           # 静态前端
│   ├── index.html
│   ├── style.css
│   ├── js/host.js
│   └── pkg/       # wasm-pack 输出（构建后生成）
├── Cargo.toml
└── build.sh
```

## 技术栈

- Rust → WASM (`wasm-bindgen`, `web-sys`)
- HTML + CSS + 少量 JS（输入采集与 RAF 循环）
- nginx 静态托管
