/**
 * Z3S Auth & Session Manager
 */

const SESSION_KEY = "z3s_auth_session";

const AuthManager = {
  getSession() {
    try {
      const data = sessionStorage.getItem(SESSION_KEY) || localStorage.getItem(SESSION_KEY);
      if (data) {
        return JSON.parse(data);
      }
    } catch (e) {
      console.error("Erro ao ler sessão:", e);
    }
    return null;
  },

  setSession(endpoint, accessKey, secretKey, region = "us-east-1", rememberMe = true) {
    const session = {
      endpoint: endpoint || window.location.origin,
      accessKey,
      secretKey,
      region,
      loggedInAt: new Date().toISOString()
    };
    const serialized = JSON.stringify(session);
    if (rememberMe) {
      localStorage.setItem(SESSION_KEY, serialized);
    } else {
      sessionStorage.setItem(SESSION_KEY, serialized);
    }
    return session;
  },

  clearSession() {
    sessionStorage.removeItem(SESSION_KEY);
    localStorage.removeItem(SESSION_KEY);
  },

  getClient() {
    const session = this.getSession();
    if (!session) return null;
    return new S3Client(session.endpoint, session.accessKey, session.secretKey, session.region);
  }
};

window.AuthManager = AuthManager;
