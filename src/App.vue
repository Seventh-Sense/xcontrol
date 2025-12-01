<template>
  <div class="app-container">
    <!-- Vue 控制的加载屏幕 -->
    <div
      v-if="showSplash"
      class="splash-screen"
      :class="{ 'fade-out': isHiding }"
    >
      <svg
        id="Layer_1"
        width="100"
        height="100"
        data-name="Layer 1"
        viewBox="0 0 144 144"
      >
        <defs>
          <clipPath id="clippath-3">
            <rect
              x="17.7"
              y="-60.3671"
              width="18.1"
              height="83.6"
              style="fill: none; stroke-width: 0px"
            >
              <animate
                attributeName="y"
                begin="0s"
                values="-60.3671;23.23;23.23;106.8334;106.8334"
                keyTimes="0;0.083;0.5;0.583;1"
                repeatCount="indefinite"
                dur="6s"
                fill="freeze"
              />
            </rect>
          </clipPath>
          <clipPath id="clippath-4">
            <rect
              x="-90.91"
              y="72"
              width="108.6"
              height="53.87"
              style="fill: none; stroke-width: 0px"
            >
              <animate
                attributeName="x"
                begin="0s"
                values="-90.91;-90.91;17.7;17.7;126.3;126.3"
                keyTimes="0;0.07;0.25;0.54;0.75;1"
                repeatCount="indefinite"
                dur="6s"
                fill="freeze"
              />
            </rect>
          </clipPath>
          <clipPath id="clippath-1">
            <rect
              x="35.8"
              y="40.65"
              width="36.2"
              height="45.28"
              style="fill: none; stroke-width: 0px"
            >
              <animate
                attributeName="x"
                begin="0s"
                values="35.8;35.8;72;72;108.2;108.2"
                keyTimes="0;0.167;0.25;0.667;0.75;1"
                repeatCount="indefinite"
                dur="6s"
                fill="freeze"
              />
            </rect>
          </clipPath>
          <clipPath id="clippath-2">
            <rect
              x="126.3"
              y="51.1"
              width="30.17"
              height="38.32"
              style="fill: none; stroke-width: 0px"
            >
              <animate
                attributeName="x"
                begin="0s"
                values="126.3;126.3;96.13;96.13;65.9667;65.9667"
                keyTimes="0;0.25;0.33;0.75;0.833;1"
                repeatCount="indefinite"
                dur="6s"
                fill="freeze"
              />
            </rect>
          </clipPath>
          <clipPath id="clippath">
            <rect
              x="108.2"
              y="18.17"
              width="70.59"
              height="39.9"
              style="fill: none; stroke-width: 0px"
            >
              <animate
                attributeName="x"
                begin="0s"
                values="108.2;108.2;37.61;37.61;-32.98;-32.98"
                keyTimes="0;0.25;0.4167;0.75;0.9167;1"
                repeatCount="indefinite"
                dur="6s"
                fill="freeze"
              />
            </rect>
          </clipPath>
          <linearGradient
            id="linear-gradient"
            x1="37.61"
            y1="72"
            x2="108.2"
            y2="72"
            gradientUnits="userSpaceOnUse"
          >
            <stop offset="0" stop-color="#66f" />
            <stop offset=".17" stop-color="#6464fa" />
            <stop offset=".34" stop-color="#605fee" />
            <stop offset=".51" stop-color="#5958d9" />
            <stop offset=".69" stop-color="#4f4dbc" />
            <stop offset=".87" stop-color="#433f97" />
            <stop offset="1" stop-color="#383477" />
          </linearGradient>
          <linearGradient
            id="linear-gradient-2"
            x1="96.13"
            y1="70.26"
            x2="126.3"
            y2="70.26"
            xlink:href="#linear-gradient"
          />
          <linearGradient
            id="linear-gradient-3"
            x1="17.7"
            y1="72"
            x2="126.3"
            y2="72"
            gradientUnits="userSpaceOnUse"
          >
            <stop offset="0" stop-color="#66f" />
            <stop offset="1" stop-color="#00ced1" />
          </linearGradient>
          <filter
            id="X_Shadow_3"
            x="-200%"
            y="-200%"
            width="400%"
            height="400%"
          >
            <feDropShadow
              dx="0"
              dy="0"
              flood-color="black"
              flood-opacity="0.4"
              stdDeviation="6"
            />
          </filter>
        </defs>

        <path
          d="M126.3,72c0-2.16-1.15-4.15-3.02-5.23-5.03-2.9-10.06-5.81-15.08-8.71v-17.42c0-2.16-1.15-4.15-3.02-5.23-9.05-5.23-18.1-10.45-27.15-15.68-3.73-2.16-8.33-2.16-12.07,0-9.45,5.46-18.9,10.91-28.36,16.37v20.9c9.45-5.46,18.9-10.91,28.36-16.37,3.73-2.16,8.33-2.16,12.07,0,3.13,1.81,11.13,6.42,18.1,10.45-8.04,4.64-16.09,9.29-24.13,13.93v20.9c8.04-4.64,16.09-9.29,24.13-13.93h0l18.1,10.45c-12.07,6.97-24.13,13.93-36.2,20.9-3.73,2.16-8.33,2.16-12.07,0-10.06-5.81-20.11-11.61-30.17-17.42V23.23c-5.03,2.9-10.06,5.81-15.08,8.71-1.87,1.08-3.02,3.07-3.02,5.22v55.73c0,2.16,1.15,4.15,3.02,5.22,15.08,8.71,30.17,17.42,45.25,26.13,3.73,2.16,8.33,2.16,12.07,0,15.08-8.71,30.17-17.42,45.25-26.13,1.87-1.08,3.02-3.07,3.02-5.22v-20.9h0s0,0,0,0Z"
          style="fill: #fff9; stroke-width: 0px; filter: url(#X_Shadow_3)"
        />

        <g style="clip-path: url(#clippath-2)">
          <path
            d="M126.3,89.42s0-15.26,0-17.42-1.15-4.15-3.02-5.23c-5.03-2.9-10.06-5.81-15.08-8.71l-12.07-6.97v20.9l30.17,17.42Z"
            style="fill: #fffe; stroke-width: 0px"
          />
        </g>
        <g style="clip-path: url(#clippath)">
          <path
            d="M78.03,40.65c-3.73-2.16-8.33-2.16-12.07,0-9.45,5.46-18.9,10.91-28.36,16.37v-20.9c9.45-5.46,18.9-10.91,28.36-16.37,3.73-2.16,8.33-2.16,12.07,0,9.05,5.23,18.1,10.45,27.15,15.68,1.87,1.08,3.02,3.07,3.02,5.23s0,17.42,0,17.42c0,0-24.13-13.93-30.17-17.42Z"
            style="fill: #fffe; stroke-width: 0px"
          />
        </g>
        <g style="clip-path: url(#clippath-1)">
          <path
            d="M105.18,66.77c-11.06,6.39-22.12,12.77-33.18,19.16v-20.9c11.06-6.39,22.12-12.77,33.18-19.16,1.87-1.08,3.02-3.07,3.02-5.23v20.9c0,2.16-1.15,4.15-3.02,5.23Z"
            style="fill: #fffe; stroke-width: 0px"
          />
        </g>
        <g style="clip-path: url(#clippath-3)">
          <path
            d="M35.8,85.93V23.23c-5.03,2.9-10.06,5.81-15.08,8.71-1.87,1.08-3.02,3.07-3.02,5.22v55.73c0,2.16,1.15,4.15,3.02,5.22,5.03,2.9,10.06,5.81,15.08,8.71v-20.9Z"
            style="fill: #fffe; stroke-width: 0px"
          />
        </g>
        <g style="clip-path: url(#clippath-4)">
          <path
            d="M78.03,124.25c15.08-8.71,30.17-17.42,45.25-26.13,1.87-1.08,3.02-3.07,3.02-5.23v-20.9c0,2.16-1.15,4.15-3.02,5.23-15.08,8.71-30.17,17.42-45.25,26.13-3.73,2.16-8.33,2.16-12.07,0-10.06-5.81-48.27-27.87-48.27-27.87,0,0,0-1.16,0,17.42,0,2.16,1.15,4.15,3.02,5.23,15.08,8.71,30.17,17.42,45.25,26.13,3.73,2.16,8.33,2.16,12.07,0Z"
            style="fill: #fffe; stroke-width: 0px"
          />
        </g>
      </svg>
      <div class="status-text" :class="{ error: hasError }">
        {{ statusMessage }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { event, window as tauriWindow } from "@tauri-apps/api";

// 状态管理
const showSplash = ref(true);
const isHiding = ref(false);
const hasError = ref(false);
const statusMessage = ref("Loading...");

// 定义事件 payload 的接口
interface ServiceErrorPayload {
  error: string;
}

// 存储取消监听的函数
let unlistenReady: (() => void) | null = null;
let unlistenError: (() => void) | null = null;

// 清理资源的函数
const cleanup = async () => {
  try {
    if (unlistenReady) {
      unlistenReady();
      unlistenReady = null;
    }
    if (unlistenError) {
      unlistenError();
      unlistenError = null;
    }
  } catch (error) {
    console.warn("清理监听器时出错:", error);
  }
};

// 跳转到服务页面
const redirectToService = () => {
  try {
    window.location.href = "http://127.0.0.1:9860";
  } catch (error) {
    console.error("跳转失败:", error);
    // 可以考虑使用 window.open 作为后备方案
    window.open("http://127.0.0.1:9860", "_self");
  }
};

// 隐藏加载屏幕
const hideSplashScreen = async () => {
  isHiding.value = true;

  // 等待淡出动画完成
  await new Promise((resolve) => setTimeout(resolve, 500));

  showSplash.value = false;
  isHiding.value = false;
};

// 处理服务就绪事件
const handleServiceReady = async (eventData: any) => {
  console.log("Received service_ready event:", eventData.payload);

  try {
    // 隐藏加载屏幕
    await hideSplashScreen();

    // 跳转到服务页面
    redirectToService();

    // 清理监听器
    await cleanup();
  } catch (error) {
    console.error("处理服务就绪事件时出错:", error);
    handleError("服务启动成功，但页面跳转失败");
  }
};

// 处理服务错误事件
const handleServiceError = async (eventData: {
  payload: ServiceErrorPayload | string;
}) => {
  console.error("Received service_error event:", eventData.payload);

  hasError.value = true;

  // 提取错误消息
  if (typeof eventData.payload === "string") {
    statusMessage.value = eventData.payload;
  } else if (eventData.payload?.error) {
    statusMessage.value = eventData.payload.error;
  } else {
    statusMessage.value = "服务启动失败，请检查日志";
  }

  // 5秒后自动关闭应用
  setTimeout(async () => {
    try {
      const currentWindow = tauriWindow.getCurrentWindow();
      await currentWindow.close();
    } catch (error) {
      console.error("关闭窗口失败:", error);
    }
  }, 5000);

  // 清理监听器
  await cleanup();
};

// 通用错误处理
const handleError = (message: string) => {
  hasError.value = true;
  statusMessage.value = message;
  console.error(message);
};

// 设置事件监听器
const setupEventListeners = async () => {
  try {
    // 监听服务就绪事件
    const readyUnlisten = await event.listen(
      "service_ready",
      handleServiceReady
    );
    unlistenReady = readyUnlisten;

    // 监听服务错误事件
    const errorUnlisten = await event.listen<ServiceErrorPayload>(
      "service_error",
      handleServiceError
    );
    unlistenError = errorUnlisten;

    console.log("事件监听器设置完成");
  } catch (error) {
    console.error("设置事件监听器失败:", error);
    handleError("初始化失败，请重启应用");
  }
};

onMounted(async () => {
  console.log("App mounted, waiting for service...");
  await setupEventListeners();
});

onUnmounted(async () => {
  console.log("App unmounting, cleaning up...");
  await cleanup();
});

// 处理页面卸载
window.addEventListener("beforeunload", () => {
  cleanup();
});
</script>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
    "Helvetica Neue", Arial, sans-serif;
}

.splash-screen {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background-color: #383477;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  z-index: 9999;
  transition: opacity 0.5s ease-out;
}

.splash-screen.fade-out {
  opacity: 0;
}

.spinner {
  width: 50px;
  height: 50px;
  border: 5px solid #f3f3f3;
  border-top: 5px solid #007aff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 20px;
}

@keyframes spin {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

.status-text {
  font-size: 16px;
  color: white;
  text-align: center;
  padding: 0 20px;
  transition: color 0.3s ease;
}

.status-text.error {
  color: #ff3b30;
}

.webview-content {
  flex: 1;
  width: 100%;
  height: 100%;
  border: none;
}
</style>
