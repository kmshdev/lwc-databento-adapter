import './style.css';
import { DemoApp } from './demo-app';

const app = new DemoApp(document);
app.mount();

declare global {
  interface Window {
    lwcDatabentoDemo?: DemoApp;
  }
}

if (import.meta.env.VITE_E2E_EXPOSE_APP === '1') window.lwcDatabentoDemo = app;
window.addEventListener(
  'pagehide',
  () => {
    void app.dispose();
  },
  { once: true },
);
