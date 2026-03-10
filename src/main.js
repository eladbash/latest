const { invoke } = window.__TAURI__.core;

document.addEventListener("DOMContentLoaded", () => {
  const appList = document.getElementById("app-list");
  appList.textContent = "Loading apps...";
});
