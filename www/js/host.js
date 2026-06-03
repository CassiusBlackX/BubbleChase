import init, {
  init as gameInit,
  resize,
  update,
  set_input,
  restart,
  get_game_status,
} from "../pkg/buble_game.js";

const canvas = document.getElementById("game-canvas");
const loading = document.getElementById("loading");
const desktopHint = document.getElementById("desktop-hint");
const mobileHint = document.getElementById("mobile-hint");
const mobileControls = document.getElementById("mobile-controls");
const restartBtn = document.getElementById("restart-btn");

const DOUBLE_TAP_MS = 320;
const LONG_PRESS_MS = 280;

let lastTs = 0;
let isMobile = false;

let controlActive = false;
let inputDx = 0;
let inputDy = 0;
let aimDx = 0;
let aimDy = 0;
let expelActive = false;

let previousTouchEndTime = 0;
let mobileMoveActive = false;
let expelGesture = false;
let longPressTimer = null;

function detectMobile() {
  return window.matchMedia("(hover: none) and (pointer: coarse)").matches;
}

function getCanvasSize() {
  const rect = canvas.getBoundingClientRect();
  return { width: rect.width, height: rect.height };
}

function directionFromCanvasPoint(clientX, clientY) {
  const rect = canvas.getBoundingClientRect();
  const cx = rect.width / 2;
  const cy = rect.height / 2;
  return {
    dx: clientX - rect.left - cx,
    dy: clientY - rect.top - cy,
  };
}

function setDirection(dx, dy) {
  inputDx = dx;
  inputDy = dy;
  aimDx = dx;
  aimDy = dy;
}

function clearDirection() {
  inputDx = 0;
  inputDy = 0;
}

function clearLongPressTimer() {
  if (longPressTimer !== null) {
    clearTimeout(longPressTimer);
    longPressTimer = null;
  }
}

function applyInput() {
  const dirX = controlActive ? inputDx : aimDx;
  const dirY = controlActive ? inputDy : aimDy;
  const active = controlActive || expelActive || mobileMoveActive;
  set_input(dirX, dirY, expelActive, active);
}

function resetInputState() {
  controlActive = false;
  expelActive = false;
  mobileMoveActive = false;
  expelGesture = false;
  clearLongPressTimer();
  clearDirection();
  aimDx = 0;
  aimDy = 0;
  updateDesktopHint();
  updateMobileHint();
}

function updateDesktopHint() {
  if (isMobile || desktopHint.classList.contains("hidden")) return;
  if (controlActive) {
    desktopHint.textContent =
      "控制中 · 朝鼠标方向移动 · 按住空格向鼠标排空 · 再点左键停止";
  } else {
    desktopHint.textContent =
      "左键点击开始朝鼠标方向移动 · 再点左键停止 · 按住空格向鼠标排空";
  }
}

function updateMobileHint() {
  if (!isMobile || mobileHint.classList.contains("hidden")) return;
  if (expelActive) {
    mobileHint.textContent = "排空模式 · 松开手指停止";
  } else if (mobileMoveActive) {
    mobileHint.textContent = "朝手指方向移动 · 双击并长按排空";
  } else {
    mobileHint.textContent = "单指触摸移动 · 连续双击并长按排空";
  }
}

function onResize() {
  const { width, height } = getCanvasSize();
  resize(width, height);
}

async function boot() {
  await init();
  const { width, height } = getCanvasSize();
  await gameInit("game-canvas", width, height);
  loading.classList.add("hidden");

  isMobile = detectMobile();
  if (isMobile) {
    mobileHint.classList.remove("hidden");
    mobileControls.classList.add("hidden");
    updateMobileHint();
  } else {
    desktopHint.classList.remove("hidden");
    updateDesktopHint();
  }

  onResize();
  applyInput();
  requestAnimationFrame(frame);
}

function frame(ts) {
  if (!lastTs) lastTs = ts;
  const dt = ts - lastTs;
  lastTs = ts;

  applyInput();
  update(dt);

  const status = get_game_status();
  if (status === 1 || status === 2) {
    restartBtn.classList.remove("hidden");
  } else {
    restartBtn.classList.add("hidden");
  }

  requestAnimationFrame(frame);
}

canvas.addEventListener("click", (e) => {
  if (isMobile) return;
  if (e.button !== 0) return;

  controlActive = !controlActive;
  if (controlActive) {
    const { dx, dy } = directionFromCanvasPoint(e.clientX, e.clientY);
    setDirection(dx, dy);
  } else {
    clearDirection();
  }
  updateDesktopHint();
  applyInput();
});

canvas.addEventListener("mousemove", (e) => {
  if (isMobile) return;
  const { dx, dy } = directionFromCanvasPoint(e.clientX, e.clientY);
  aimDx = dx;
  aimDy = dy;
  if (controlActive) {
    inputDx = dx;
    inputDy = dy;
  }
  applyInput();
});

window.addEventListener("keydown", (e) => {
  if (e.code === "Space") {
    e.preventDefault();
    expelActive = true;
    applyInput();
  }
  if (e.code === "KeyR") {
    restart();
    resetInputState();
    applyInput();
  }
});

window.addEventListener("keyup", (e) => {
  if (e.code === "Space") {
    expelActive = false;
    applyInput();
  }
});

restartBtn.addEventListener("click", () => {
  restart();
  resetInputState();
  applyInput();
});

function updateTouchDirection(touch) {
  const { dx, dy } = directionFromCanvasPoint(touch.clientX, touch.clientY);
  setDirection(dx, dy);
  applyInput();
}

canvas.addEventListener(
  "touchstart",
  (e) => {
    if (!isMobile) return;
    if (e.target !== canvas) return;
    e.preventDefault();

    const now = Date.now();
    const isDoubleTap = now - previousTouchEndTime < DOUBLE_TAP_MS;
    const touch = e.touches[0];

    clearLongPressTimer();

    if (isDoubleTap) {
      expelGesture = true;
      mobileMoveActive = false;
      expelActive = false;
      updateTouchDirection(touch);
      longPressTimer = setTimeout(() => {
        expelActive = true;
        updateMobileHint();
        applyInput();
      }, LONG_PRESS_MS);
    } else {
      expelGesture = false;
      expelActive = false;
      mobileMoveActive = true;
      updateTouchDirection(touch);
    }

    updateMobileHint();
  },
  { passive: false }
);

canvas.addEventListener(
  "touchmove",
  (e) => {
    if (!isMobile) return;
    if (e.target !== canvas) return;
    e.preventDefault();

    const touch = e.touches[0];
    if (mobileMoveActive || expelGesture) {
      updateTouchDirection(touch);
    }
  },
  { passive: false }
);

canvas.addEventListener(
  "touchend",
  (e) => {
    if (!isMobile) return;
    e.preventDefault();

    previousTouchEndTime = Date.now();
    clearLongPressTimer();
    mobileMoveActive = false;
    expelActive = false;
    expelGesture = false;
    clearDirection();
    updateMobileHint();
    applyInput();
  },
  { passive: false }
);

canvas.addEventListener(
  "touchcancel",
  (e) => {
    if (!isMobile) return;
    e.preventDefault();
    previousTouchEndTime = Date.now();
    clearLongPressTimer();
    mobileMoveActive = false;
    expelActive = false;
    expelGesture = false;
    clearDirection();
    updateMobileHint();
    applyInput();
  },
  { passive: false }
);

window.addEventListener("resize", onResize);
window.addEventListener("orientationchange", () => setTimeout(onResize, 200));

boot().catch((err) => {
  loading.textContent = "加载失败: " + err;
  console.error(err);
});
