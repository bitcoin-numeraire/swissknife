import { preferredLanguage, translate } from './languages.mjs';

const form = document.querySelector('#connect');
const instance = document.querySelector('#instance');
const login = document.querySelector('#login');
const error = document.querySelector('#error');
const submit = form.querySelector('button');
const hosted = 'https://app.numeraire.tech';
const hostedLogin = 'https://auth.numeraire.tech';

const language = document.querySelector('#language');
const labels = Array.from(document.querySelectorAll('[data-i18n]'), (element) => [element, element.textContent]);
let locale;
let errorMessage = '';

function applyLanguage() {
  locale = language.value || preferredLanguage(navigator.languages);
  document.documentElement.lang = locale === 'pt' ? 'pt-PT' : locale;
  for (const [element, source] of labels) element.textContent = translate(locale, source);
  error.textContent = translate(locale, errorMessage);
}

language.value = localStorage.getItem('launcherLanguage') || '';
if (language.selectedIndex < 0) language.value = '';
applyLanguage();
language.addEventListener('change', () => {
  if (language.value) localStorage.setItem('launcherLanguage', language.value);
  else localStorage.removeItem('launcherLanguage');
  applyLanguage();
});
window.addEventListener('languagechange', applyLanguage);

instance.value = localStorage.getItem('instance') || hosted;
login.value = localStorage.getItem('login') ?? (instance.value === hosted ? hostedLogin : '');
instance.addEventListener('input', () => {
  if (login.value === hostedLogin && instance.value !== hosted) login.value = '';
  if (!login.value && instance.value === hosted) login.value = hostedLogin;
});

form.addEventListener('submit', async (event) => {
  event.preventDefault();
  errorMessage = '';
  error.textContent = '';
  submit.disabled = true;
  try {
    await window.__TAURI__.core.invoke('connect', { instance: instance.value.trim(), login: login.value.trim() });
    localStorage.setItem('instance', instance.value.trim());
    localStorage.setItem('login', login.value.trim());
  } catch (reason) {
    errorMessage = String(reason);
    error.textContent = translate(locale, errorMessage);
  } finally {
    submit.disabled = false;
  }
});
