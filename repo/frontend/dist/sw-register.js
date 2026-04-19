// Registers the service worker as soon as the page finishes loading.
// Scope "/" lets it intercept all same-origin requests including /api/*.
if ('serviceWorker' in navigator) {
    window.addEventListener('load', function () {
        navigator.serviceWorker
            .register('/sw.js', { scope: '/' })
            .catch(function (err) {
                console.warn('SW registration failed:', err);
            });
    });
}
