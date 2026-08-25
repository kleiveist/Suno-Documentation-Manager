import { SunoDocumentationApp } from "./app";
import { createDesktopApi } from "./api/desktop";
import "./styles/index.css";

const PRODUCT_NAME = "Suno Documentation Manager";
document.title = PRODUCT_NAME;

const root = document.querySelector<HTMLDivElement>("#app");

if (!root) throw new Error("App-Container #app fehlt");

new SunoDocumentationApp(root, createDesktopApi()).start();
