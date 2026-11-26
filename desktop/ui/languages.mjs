// The launcher HTML and native validation errors supply the English source text.
export const translations = {
  'Language': {
    de: 'Sprache',
    es: 'Idioma',
    fr: 'Langue',
    it: 'Lingua',
    pt: 'Idioma',
  },
  'System language': {
    de: 'Systemsprache',
    es: 'Idioma del sistema',
    fr: 'Langue du système',
    it: 'Lingua di sistema',
    pt: 'Idioma do sistema',
  },
  'Connect to SwissKnife': {
    de: 'Mit SwissKnife verbinden',
    es: 'Conectarse a SwissKnife',
    fr: 'Se connecter à SwissKnife',
    it: 'Connettersi a SwissKnife',
    pt: 'Ligar ao SwissKnife',
  },
  'Dashboard URL': {
    de: 'Dashboard-URL',
    es: 'URL del panel',
    fr: 'URL du tableau de bord',
    it: 'URL della dashboard',
    pt: 'URL do painel',
  },
  'Single sign-on': {
    de: 'Single Sign-on',
    es: 'Inicio de sesión único',
    fr: 'Authentification unique',
    it: 'Accesso singolo',
    pt: 'Início de sessão único',
  },
  'Identity provider URL': {
    de: 'URL des Identitätsanbieters',
    es: 'URL del proveedor de identidad',
    fr: 'URL du fournisseur d’identité',
    it: 'URL del provider di identità',
    pt: 'URL do fornecedor de identidade',
  },
  'Enter your provider’s origin if your instance uses single sign-on.': {
    de: 'Wenn Ihre Instanz Single Sign-on verwendet, geben Sie die Basis-URL Ihres Identitätsanbieters ein.',
    es: 'Si tu instancia utiliza el inicio de sesión único, introduce la URL base de tu proveedor de identidad.',
    fr: 'Si votre instance utilise l’authentification unique, saisissez l’URL de base de votre fournisseur d’identité.',
    it: 'Se la tua istanza usa l’accesso singolo, inserisci l’URL di base del provider di identità.',
    pt: 'Se a sua instância utilizar o início de sessão único, introduza o URL base do seu fornecedor de identidade.',
  },
  'Open wallet': {
    de: 'Wallet öffnen',
    es: 'Abrir cartera',
    fr: 'Ouvrir le portefeuille',
    it: 'Apri portafoglio',
    pt: 'Abrir carteira',
  },
  'Enter a complete instance URL.': {
    de: 'Geben Sie die vollständige URL der Instanz ein.',
    es: 'Introduce la URL completa de la instancia.',
    fr: 'Saisissez l’URL complète de l’instance.',
    it: 'Inserisci l’URL completo dell’istanza.',
    pt: 'Introduza o URL completo da instância.',
  },
  'Enter an instance hostname.': {
    de: 'Geben Sie den Hostnamen der Instanz ein.',
    es: 'Introduce el nombre de host de la instancia.',
    fr: 'Saisissez le nom d’hôte de l’instance.',
    it: 'Inserisci il nome host dell’istanza.',
    pt: 'Introduza o nome do servidor da instância.',
  },
  'Use HTTPS or a local loopback HTTP address.': {
    de: 'Verwenden Sie HTTPS oder eine lokale HTTP-Loopback-Adresse.',
    es: 'Usa HTTPS o una dirección HTTP de bucle local.',
    fr: 'Utilisez HTTPS ou une adresse HTTP de bouclage local.',
    it: 'Usa HTTPS o un indirizzo HTTP di loopback locale.',
    pt: 'Utilize HTTPS ou um endereço HTTP de loopback local.',
  },
  'This address is reserved for the desktop application.': {
    de: 'Diese Adresse ist für die Desktop-Anwendung reserviert.',
    es: 'Esta dirección está reservada para la aplicación de escritorio.',
    fr: 'Cette adresse est réservée à l’application de bureau.',
    it: 'Questo indirizzo è riservato all’applicazione desktop.',
    pt: 'Este endereço está reservado para a aplicação de computador.',
  },
  'Use an origin without credentials, a path, a query or a fragment.': {
    de: 'Geben Sie eine Basis-URL ohne Zugangsdaten, Pfad, Abfrageparameter oder Fragment ein.',
    es: 'Introduce una URL base sin credenciales, ruta, parámetros de consulta ni fragmento.',
    fr: 'Saisissez une URL de base sans identifiants, chemin, paramètres de requête ni fragment.',
    it: 'Inserisci un URL di base senza credenziali, percorso, parametri di query o frammento.',
    pt: 'Introduza um URL base sem credenciais, caminho, parâmetros de consulta ou fragmento.',
  },
  'Open the connection from the desktop launcher.': {
    de: 'Stellen Sie die Verbindung über das Startfenster der Desktop-Anwendung her.',
    es: 'Abre la conexión desde la pantalla de inicio de la aplicación de escritorio.',
    fr: 'Ouvrez la connexion depuis l’écran d’accueil de l’application de bureau.',
    it: 'Avvia la connessione dalla schermata iniziale dell’applicazione desktop.',
    pt: 'Abra a ligação a partir do ecrã inicial da aplicação de computador.',
  },
};

const supportedLanguages = ['de', 'en', 'es', 'fr', 'it', 'pt'];

export function preferredLanguage(languages) {
  for (const tag of languages) {
    const language = tag.toLowerCase().split('-')[0];
    if (supportedLanguages.includes(language)) return language;
  }
  return 'en';
}

export function translate(language, source) {
  return translations[source]?.[language] ?? source;
}
