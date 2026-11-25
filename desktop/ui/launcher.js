const form = document.querySelector('#connect');
const instance = document.querySelector('#instance');
const login = document.querySelector('#login');
const error = document.querySelector('#error');
const submit = form.querySelector('button');
const hosted = 'https://app.numeraire.tech';
const hostedLogin = 'https://auth.numeraire.tech';

instance.value = localStorage.getItem('instance') || hosted;
login.value = localStorage.getItem('login') ?? (instance.value === hosted ? hostedLogin : '');
instance.addEventListener('input', () => {
  if (login.value === hostedLogin && instance.value !== hosted) login.value = '';
  if (!login.value && instance.value === hosted) login.value = hostedLogin;
});

form.addEventListener('submit', async (event) => {
  event.preventDefault();
  error.textContent = '';
  submit.disabled = true;
  try {
    await window.__TAURI__.core.invoke('connect', { instance: instance.value.trim(), login: login.value.trim() });
    localStorage.setItem('instance', instance.value.trim());
    localStorage.setItem('login', login.value.trim());
  } catch (reason) {
    error.textContent = String(reason);
  } finally {
    submit.disabled = false;
  }
});
