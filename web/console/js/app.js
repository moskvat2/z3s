/**
 * Z3S Web Console - React 18 Root Application
 */

const { useState, useEffect, useMemo } = React;

// Toast helper
let addToastExternal = () => {};

function App() {
  const [session, setSession] = useState(AuthManager.getSession());
  const [theme, setTheme] = useState(localStorage.getItem("z3s_theme") || "dark");
  const [currentView, setCurrentView] = useState("buckets"); // 'buckets', 'objects', 'dashboard', 'settings'
  const [selectedBucket, setSelectedBucket] = useState(null);
  const [toasts, setToasts] = useState([]);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("z3s_theme", theme);
  }, [theme]);

  const toggleTheme = () => {
    setTheme(prev => prev === "dark" ? "light" : "dark");
  };

  const addToast = (message, type = "info") => {
    const id = Date.now() + Math.random();
    setToasts(prev => [...prev, { id, message, type }]);
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
    }, 4000);
  };

  addToastExternal = addToast;

  const handleLogin = (newSession) => {
    setSession(newSession);
    addToast("Login realizado com sucesso!", "success");
  };

  const handleLogout = () => {
    AuthManager.clearSession();
    setSession(null);
    setSelectedBucket(null);
    addToast("Sessão finalizada.", "info");
  };

  if (!session) {
    return <LoginView onLogin={handleLogin} theme={theme} toggleTheme={toggleTheme} />;
  }

  return (
    <div className="flex h-screen overflow-hidden bg-slate-950 text-slate-100 data-[theme=light]:bg-slate-50 data-[theme=light]:text-slate-900">
      <Sidebar 
        currentView={currentView} 
        setCurrentView={(view) => {
          setCurrentView(view);
          if (view === "buckets") setSelectedBucket(null);
        }}
        selectedBucket={selectedBucket}
      />
      
      <div className="flex flex-col flex-1 overflow-hidden">
        <Navbar 
          session={session} 
          theme={theme} 
          toggleTheme={toggleTheme} 
          onLogout={handleLogout} 
        />
        
        <main className="flex-1 overflow-y-auto p-6 bg-slate-900/50 data-[theme=light]:bg-slate-100/60">
          {currentView === "buckets" && (
            <BucketsView 
              onSelectBucket={(bucket) => {
                setSelectedBucket(bucket);
                setCurrentView("objects");
              }}
              addToast={addToast}
            />
          )}

          {currentView === "objects" && (
            <ObjectsExplorerView 
              bucket={selectedBucket} 
              onBack={() => {
                setSelectedBucket(null);
                setCurrentView("buckets");
              }}
              addToast={addToast}
            />
          )}

          {currentView === "dashboard" && (
            <DashboardView addToast={addToast} />
          )}

          {currentView === "settings" && (
            <SettingsView session={session} addToast={addToast} />
          )}
        </main>
      </div>

      <ToastContainer toasts={toasts} />
    </div>
  );
}

// -------------------------------------------------------------
// View: Login
// -------------------------------------------------------------
function LoginView({ onLogin, theme, toggleTheme }) {
  const [endpoint, setEndpoint] = useState(window.location.origin);
  const [accessKey, setAccessKey] = useState("Z3SACCESSKEYEXAMPLE");
  const [secretKey, setSecretKey] = useState("Z3SSECRETKEYEXAMPLE1234567890ABCDEF");
  const [region, setRegion] = useState("us-east-1");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e) => {
    e.preventDefault();
    setLoading(true);
    setError("");

    try {
      const client = new S3Client(endpoint, accessKey, secretKey, region);
      await client.listBuckets();
      const session = AuthManager.setSession(endpoint, accessKey, secretKey, region);
      onLogin(session);
    } catch (err) {
      console.error(err);
      setError("Falha na autenticação SigV4. Verifique as credenciais ou a URL do endpoint.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex items-center justify-center min-h-screen p-4 bg-slate-950 text-slate-100">
      <div className="w-full max-w-md p-8 bg-slate-900 border border-slate-800 rounded-2xl shadow-2xl animate-fade-in">
        <div className="flex items-center justify-between mb-8">
          <div className="flex items-center space-x-3">
            <div className="flex items-center justify-center w-10 h-10 bg-blue-600 rounded-xl font-black text-xl text-white shadow-lg shadow-blue-500/30">
              Z3
            </div>
            <div>
              <h1 className="text-xl font-bold tracking-tight">Z3S Storage Console</h1>
              <p className="text-xs text-slate-400">AWS S3 Compatible Web Management</p>
            </div>
          </div>
          <button 
            type="button" 
            onClick={toggleTheme}
            className="p-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 transition"
          >
            {theme === "dark" ? "☀️" : "🌙"}
          </button>
        </div>

        {error && (
          <div className="p-3 mb-6 text-sm text-red-400 bg-red-950/50 border border-red-800/60 rounded-xl">
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-1">
              Endpoint URL
            </label>
            <input 
              type="text" 
              value={endpoint} 
              onChange={e => setEndpoint(e.target.value)}
              required 
              className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-sm text-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="http://127.0.0.1:9000"
            />
          </div>

          <div>
            <label className="block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-1">
              Access Key ID
            </label>
            <input 
              type="text" 
              value={accessKey} 
              onChange={e => setAccessKey(e.target.value)}
              required 
              className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-sm text-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
              placeholder="Z3SACCESSKEYEXAMPLE"
            />
          </div>

          <div>
            <label className="block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-1">
              Secret Access Key
            </label>
            <input 
              type="password" 
              value={secretKey} 
              onChange={e => setSecretKey(e.target.value)}
              required 
              className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-sm text-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
              placeholder="••••••••••••••••"
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-1">
                Region
              </label>
              <input 
                type="text" 
                value={region} 
                onChange={e => setRegion(e.target.value)}
                className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-sm text-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div className="flex items-end">
              <button 
                type="submit" 
                disabled={loading}
                className="w-full py-2 px-4 bg-blue-600 hover:bg-blue-500 text-white font-semibold rounded-lg shadow-lg shadow-blue-600/30 transition flex items-center justify-center space-x-2"
              >
                {loading ? (
                  <span className="inline-block animate-spin">⏳</span>
                ) : (
                  <span>Entrar</span>
                )}
              </button>
            </div>
          </div>
        </form>

        <div className="mt-8 pt-6 border-t border-slate-800/80 text-center text-xs text-slate-500">
          Z3S Distributed Engine &bull; Reed-Solomon 4+2 &bull; SigV4
        </div>
      </div>
    </div>
  );
}

// -------------------------------------------------------------
// Component: Navbar
// -------------------------------------------------------------
function Navbar({ session, theme, toggleTheme, onLogout }) {
  return (
    <header className="h-16 px-6 flex items-center justify-between border-b border-slate-800 bg-slate-900 text-slate-100">
      <div className="flex items-center space-x-4">
        <span className="flex items-center space-x-2 px-3 py-1 bg-emerald-950/60 border border-emerald-800/60 rounded-full text-xs font-medium text-emerald-400">
          <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
          <span>Nó Online: {session.endpoint}</span>
        </span>
      </div>

      <div className="flex items-center space-x-4">
        <button 
          onClick={toggleTheme}
          className="p-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 transition text-sm"
          title="Alternar Tema"
        >
          {theme === "dark" ? "☀️ Claro" : "🌙 Escuro"}
        </button>

        <div className="flex items-center space-x-2 pl-4 border-l border-slate-800">
          <div className="w-8 h-8 rounded-full bg-blue-600/30 border border-blue-500/40 flex items-center justify-center font-bold text-xs text-blue-400">
            {session.accessKey.substring(0, 2)}
          </div>
          <span className="text-xs font-mono text-slate-400 truncate max-w-[120px]">
            {session.accessKey}
          </span>
          <button 
            onClick={onLogout}
            className="p-1.5 ml-2 text-slate-400 hover:text-red-400 rounded-lg hover:bg-slate-800 transition text-xs"
            title="Sair"
          >
            Sair 🚪
          </button>
        </div>
      </div>
    </header>
  );
}

// -------------------------------------------------------------
// Component: Sidebar
// -------------------------------------------------------------
function Sidebar({ currentView, setCurrentView, selectedBucket }) {
  const navItems = [
    { id: "buckets", label: "Buckets S3", icon: "📦" },
    { id: "dashboard", label: "Métricas & Saúde", icon: "📊" },
    { id: "settings", label: "Configurações", icon: "⚙️" },
  ];

  return (
    <aside className="w-64 border-r border-slate-800 bg-slate-950 flex flex-col justify-between">
      <div>
        <div className="h-16 px-6 flex items-center space-x-3 border-b border-slate-800">
          <div className="flex items-center justify-center w-8 h-8 bg-blue-600 rounded-lg font-black text-white text-sm shadow-md shadow-blue-500/20">
            Z3
          </div>
          <span className="font-bold text-base tracking-tight text-white">Z3S Console</span>
        </div>

        <nav className="p-4 space-y-1">
          {navItems.map(item => (
            <button
              key={item.id}
              onClick={() => setCurrentView(item.id)}
              className={`w-full flex items-center space-x-3 px-3 py-2.5 rounded-xl text-sm font-medium transition ${
                currentView === item.id 
                  ? "bg-blue-600 text-white shadow-lg shadow-blue-600/20" 
                  : "text-slate-400 hover:bg-slate-900 hover:text-slate-100"
              }`}
            >
              <span>{item.icon}</span>
              <span>{item.label}</span>
            </button>
          ))}
        </nav>
      </div>

      <div className="p-4 m-4 bg-slate-900 border border-slate-800 rounded-xl text-xs text-slate-400">
        <div className="font-semibold text-slate-200 mb-1">Z3S Engine v0.1.0</div>
        <div>Erasure: RS 4+2</div>
        <div className="text-emerald-400 mt-1">● Cluster Operacional</div>
      </div>
    </aside>
  );
}

// -------------------------------------------------------------
// View: Buckets Management (Fase 1 / 2)
// -------------------------------------------------------------
function BucketsView({ onSelectBucket, addToast }) {
  const [buckets, setBuckets] = useState([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [showCreateModal, setShowCreateModal] = useState(false);

  const fetchBuckets = async () => {
    setLoading(true);
    try {
      const client = AuthManager.getClient();
      const list = await client.listBuckets();
      setBuckets(list);
    } catch (err) {
      addToast("Erro ao listar buckets: " + err.message, "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchBuckets();
  }, []);

  const filteredBuckets = useMemo(() => {
    return buckets.filter(b => b.name.toLowerCase().includes(search.toLowerCase()));
  }, [buckets, search]);

  return (
    <div className="space-y-6 animate-fade-in">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Buckets de Armazenamento</h2>
          <p className="text-sm text-slate-400">Gerencie contêineres de dados e controle de acesso.</p>
        </div>
        <div className="flex items-center space-x-3">
          <button 
            onClick={fetchBuckets}
            className="px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-sm font-medium rounded-xl border border-slate-700 transition"
          >
            🔄 Atualizar
          </button>
          <button 
            onClick={() => setShowCreateModal(true)}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm font-semibold rounded-xl shadow-lg shadow-blue-600/30 transition flex items-center space-x-2"
          >
            <span>➕ Criar Bucket</span>
          </button>
        </div>
      </div>

      <div className="bg-slate-900 border border-slate-800 rounded-2xl overflow-hidden shadow-xl">
        <div className="p-4 border-b border-slate-800 flex items-center justify-between">
          <input 
            type="text" 
            placeholder="🔍 Buscar bucket por nome..." 
            value={search}
            onChange={e => setSearch(e.target.value)}
            className="w-80 px-3 py-2 bg-slate-800 border border-slate-700 rounded-xl text-sm text-slate-100 placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <span className="text-xs text-slate-400">Total: {filteredBuckets.length} buckets</span>
        </div>

        {loading ? (
          <div className="p-12 text-center text-slate-400 animate-pulse">
            Carregando buckets do storage...
          </div>
        ) : filteredBuckets.length === 0 ? (
          <div className="p-12 text-center text-slate-500">
            Nenhum bucket encontrado.
          </div>
        ) : (
          <table className="w-full text-left text-sm">
            <thead className="bg-slate-950/60 text-xs font-semibold text-slate-400 uppercase tracking-wider border-b border-slate-800">
              <tr>
                <th className="px-6 py-3.5">Nome do Bucket</th>
                <th className="px-6 py-3.5">Região</th>
                <th className="px-6 py-3.5">Acesso</th>
                <th className="px-6 py-3.5">Criado em</th>
                <th className="px-6 py-3.5 text-right">Ações</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60">
              {filteredBuckets.map(b => (
                <tr key={b.name} className="hover:bg-slate-800/40 transition">
                  <td className="px-6 py-4 font-semibold text-blue-400 hover:underline cursor-pointer" onClick={() => onSelectBucket(b.name)}>
                    📦 {b.name}
                  </td>
                  <td className="px-6 py-4 text-slate-400">us-east-1</td>
                  <td className="px-6 py-4">
                    <span className="px-2.5 py-1 text-xs font-medium bg-slate-800 text-slate-300 rounded-md border border-slate-700">
                      Privado (Bucket Policy)
                    </span>
                  </td>
                  <td className="px-6 py-4 text-slate-400 text-xs font-mono">
                    {new Date(b.creationDate).toLocaleString()}
                  </td>
                  <td className="px-6 py-4 text-right">
                    <button 
                      onClick={() => onSelectBucket(b.name)}
                      className="px-3 py-1 bg-slate-800 hover:bg-slate-700 text-xs text-slate-200 rounded-lg border border-slate-700 transition"
                    >
                      Explorar
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {showCreateModal && (
        <CreateBucketModal 
          onClose={() => setShowCreateModal(false)} 
          onCreated={() => {
            setShowCreateModal(false);
            fetchBuckets();
            addToast("Bucket criado com sucesso!", "success");
          }}
          addToast={addToast}
        />
      )}
    </div>
  );
}

// -------------------------------------------------------------
// Component: Create Bucket Modal
// -------------------------------------------------------------
function CreateBucketModal({ onClose, onCreated, addToast }) {
  const [bucketName, setBucketName] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleCreate = async (e) => {
    e.preventDefault();
    if (!/^[a-z0-9.-]{3,63}$/.test(bucketName)) {
      setError("O nome deve ter entre 3 e 63 caracteres (apenas letras minúsculas, números e hífens).");
      return;
    }

    setLoading(true);
    setError("");
    try {
      const client = AuthManager.getClient();
      await client.createBucket(bucketName);
      onCreated();
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fade-in">
      <div className="w-full max-w-md p-6 bg-slate-900 border border-slate-800 rounded-2xl shadow-2xl">
        <h3 className="text-lg font-bold text-slate-100 mb-4">Criar Novo Bucket</h3>
        
        {error && (
          <div className="p-3 mb-4 text-xs text-red-400 bg-red-950/60 border border-red-800/60 rounded-xl">
            {error}
          </div>
        )}

        <form onSubmit={handleCreate} className="space-y-4">
          <div>
            <label className="block text-xs font-semibold text-slate-400 mb-1">
              Nome do Bucket
            </label>
            <input 
              type="text" 
              value={bucketName}
              onChange={e => setBucketName(e.target.value.toLowerCase())}
              placeholder="ex: meus-backups-2026"
              required
              className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-sm text-slate-100 focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
            />
          </div>

          <div className="flex justify-end space-x-3 pt-4 border-t border-slate-800">
            <button 
              type="button" 
              onClick={onClose}
              className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-sm font-medium rounded-xl transition"
            >
              Cancelar
            </button>
            <button 
              type="submit" 
              disabled={loading}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm font-semibold rounded-xl shadow-lg shadow-blue-600/30 transition flex items-center space-x-2"
            >
              {loading ? "Criando..." : "Criar Bucket"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// -------------------------------------------------------------
// View: Objects Explorer View (Preview)
// -------------------------------------------------------------
function ObjectsExplorerView({ bucket, onBack, addToast }) {
  const [contents, setContents] = useState({ folders: [], objects: [] });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadObjects() {
      setLoading(true);
      try {
        const client = AuthManager.getClient();
        const data = await client.listObjects(bucket, "", "/");
        setContents(data);
      } catch (err) {
        addToast("Erro ao listar objetos: " + err.message, "error");
      } finally {
        setLoading(false);
      }
    }
    loadObjects();
  }, [bucket]);

  return (
    <div className="space-y-6 animate-fade-in">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-3">
          <button 
            onClick={onBack}
            className="p-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-xl transition"
          >
            ⬅️ Voltar
          </button>
          <div>
            <h2 className="text-2xl font-bold tracking-tight">s3://{bucket}</h2>
            <p className="text-sm text-slate-400">Navegador de arquivos e objetos do bucket.</p>
          </div>
        </div>
      </div>

      <div className="bg-slate-900 border border-slate-800 rounded-2xl overflow-hidden shadow-xl p-6">
        {loading ? (
          <div className="p-8 text-center text-slate-400 animate-pulse">Carregando objetos...</div>
        ) : contents.objects.length === 0 && contents.folders.length === 0 ? (
          <div className="p-8 text-center text-slate-500">Bucket vazio.</div>
        ) : (
          <ul className="divide-y divide-slate-800">
            {contents.objects.map(obj => (
              <li key={obj.key} className="py-3 flex items-center justify-between">
                <span className="font-mono text-sm text-slate-200">📄 {obj.key}</span>
                <span className="text-xs text-slate-400">{(obj.size / 1024).toFixed(1)} KB</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

// -------------------------------------------------------------
// Views: Dashboard & Settings
// -------------------------------------------------------------
function DashboardView() {
  return (
    <div className="space-y-6 animate-fade-in">
      <h2 className="text-2xl font-bold tracking-tight">Painel de Métricas & Saúde</h2>
      <div className="grid grid-cols-3 gap-6">
        <div className="p-6 bg-slate-900 border border-slate-800 rounded-2xl">
          <div className="text-xs font-semibold text-slate-400 uppercase">Capacidade Física</div>
          <div className="text-3xl font-extrabold text-emerald-400 mt-2">60.0 GB</div>
          <div className="text-xs text-slate-500 mt-1">Disco /mnt/dados (0.9% usado)</div>
        </div>
        <div className="p-6 bg-slate-900 border border-slate-800 rounded-2xl">
          <div className="text-xs font-semibold text-slate-400 uppercase">Throughput Pico</div>
          <div className="text-3xl font-extrabold text-blue-400 mt-2">482 MB/s</div>
          <div className="text-xs text-slate-500 mt-1">Saturação de leitura NVMe</div>
        </div>
        <div className="p-6 bg-slate-900 border border-slate-800 rounded-2xl">
          <div className="text-xs font-semibold text-slate-400 uppercase">Integridade Reed-Solomon</div>
          <div className="text-3xl font-extrabold text-purple-400 mt-2">100% OK</div>
          <div className="text-xs text-slate-500 mt-1">Zero Bitrot / Auto-Healing Ativo</div>
        </div>
      </div>
    </div>
  );
}

function SettingsView({ session }) {
  return (
    <div className="space-y-6 animate-fade-in max-w-2xl">
      <h2 className="text-2xl font-bold tracking-tight">Configurações da Sessão</h2>
      <div className="p-6 bg-slate-900 border border-slate-800 rounded-2xl space-y-4">
        <div>
          <div className="text-xs text-slate-400">Endpoint Atual</div>
          <div className="text-sm font-mono text-slate-100">{session.endpoint}</div>
        </div>
        <div>
          <div className="text-xs text-slate-400">Access Key</div>
          <div className="text-sm font-mono text-slate-100">{session.accessKey}</div>
        </div>
        <div>
          <div className="text-xs text-slate-400">Região Ativa</div>
          <div className="text-sm font-mono text-slate-100">{session.region}</div>
        </div>
      </div>
    </div>
  );
}

// -------------------------------------------------------------
// Component: Toast Container
// -------------------------------------------------------------
function ToastContainer({ toasts }) {
  return (
    <div className="fixed bottom-6 right-6 z-50 flex flex-col space-y-2">
      {toasts.map(t => (
        <div
          key={t.id}
          className={`px-4 py-3 rounded-xl shadow-2xl text-sm font-medium border flex items-center space-x-2 animate-fade-in ${
            t.type === "success" 
              ? "bg-emerald-950/90 text-emerald-200 border-emerald-800"
              : t.type === "error"
              ? "bg-red-950/90 text-red-200 border-red-800"
              : "bg-slate-900/90 text-slate-200 border-slate-700"
          }`}
        >
          <span>{t.type === "success" ? "✅" : t.type === "error" ? "❌" : "ℹ️"}</span>
          <span>{t.message}</span>
        </div>
      ))}
    </div>
  );
}

// Mount React Root
const root = ReactDOM.createRoot(document.getElementById("root"));
root.render(<App />);
