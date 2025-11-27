import { createApp } from "vue";
import App from "./App.vue";

createApp(App)
  .mount("#app")
  .$nextTick(() => {
    // 确保 App 已经挂载到 DOM
    postMessage({ payload: "removeLoading" }, "*");
  });
