// No service worker is used by the dashboard. This file prevents stale browser
// registrations from repeatedly requesting a missing /service-worker.js.
self.addEventListener('install', () => {
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.registration.unregister());
});
