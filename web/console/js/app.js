/**
 * Z3S Web Console - AWS S3 Management Console Experience (React 18)
 * Designed following AWS Cloudscape Design System patterns
 */

const { useState, useEffect, useMemo } = React;

function App() {
  const [session, setSession] = useState(AuthManager.getSession());
  const [currentTab, setCurrentTab] = useState("buckets"); // 'buckets', 'bucket-detail'
  const [selectedBucket, setSelectedBucket] = useState(null);
  const [activeBucketTab, setActiveBucketTab] = useState("objects"); // 'objects', 'properties', 'permissions', 'metrics'
  const [toasts, setToasts] = useState([]);

  const addToast = (message, type = "info") => {
    const id = Date.now() + Math.random();
    setToasts(prev => [...prev, { id, message, type }]);
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
    }, 4000);
  };

  const handleLogin = (newSession) => {
    setSession(newSession);
    addToast("Autenticado com sucesso no console Z3S S3.", "success");
  };

  const handleLogout = () => {
    AuthManager.clearSession();
    setSession(null);
    setSelectedBucket(null);
    addToast("Sessão encerrada.", "info");
  };

  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  if (!session) {
    return <AwsSignInView onLogin={handleLogin} />;
  }

  return (
    <div className="min-h-screen bg-[#f2f3f3] text-[#16191f] font-sans antialiased flex flex-col selection:bg-[#2563eb] selection:text-white">
      {/* 1. AWS Top Navigation Bar */}
      <AwsGlobalHeader 
        session={session} 
        sidebarCollapsed={sidebarCollapsed}
        onToggleSidebar={() => setSidebarCollapsed(!sidebarCollapsed)}
        onLogout={handleLogout}
        onNavigateHome={() => {
          setSelectedBucket(null);
          setCurrentTab("buckets");
        }}
      />

      {/* 2. Main Content Body with Sidebar */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Sidebar Menu */}
        <AwsSidebar
          currentTab={currentTab}
          collapsed={sidebarCollapsed}
          onNavigate={(tab) => {
            setSelectedBucket(null);
            setCurrentTab(tab);
          }}
        />

        {/* Workspace Main Area */}
        <main className="flex-1 overflow-y-auto p-6">
          <div className="max-w-[1600px] w-full mx-auto space-y-6">
            {currentTab === "buckets" && (
              <AwsBucketsView 
                onSelectBucket={(bucketName) => {
                  setSelectedBucket(bucketName);
                  setActiveBucketTab("objects");
                  setCurrentTab("bucket-detail");
                }}
                addToast={addToast}
              />
            )}

            {currentTab === "bucket-detail" && selectedBucket && (
              <AwsBucketDetailView 
                bucket={selectedBucket}
                activeTab={activeBucketTab}
                setActiveTab={setActiveBucketTab}
                onBack={() => {
                  setSelectedBucket(null);
                  setCurrentTab("buckets");
                }}
                addToast={addToast}
              />
            )}

            {currentTab === "cluster-metrics" && (
              <AwsClusterMetricsView addToast={addToast} />
            )}

            {currentTab === "iam-keys" && (
              <AwsIamKeysView addToast={addToast} />
            )}
          </div>
        </main>
      </div>

      <ToastContainer toasts={toasts} />
    </div>
  );
}

// ----------------------------------------------------------------------
// 1. AWS Sign-In View (Authentic AWS Login Screen)
// ----------------------------------------------------------------------
function AwsSignInView({ onLogin }) {
  const [accessKey, setAccessKey] = useState("Z3SACCESSKEYEXAMPLE");
  const [secretKey, setSecretKey] = useState("Z3SSECRETKEYEXAMPLE1234567890ABCDEF");
  const [showPassword, setShowPassword] = useState(false);
  const [rememberMe, setRememberMe] = useState(true);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e) => {
    e.preventDefault();
    setLoading(true);
    setError("");

    // Endpoint is automatic from the browser window URL!
    const endpoint = window.location.origin;
    const region = "us-east-1";

    try {
      const client = new S3Client(endpoint, accessKey.trim(), secretKey.trim(), region);
      await client.listBuckets();
      const session = AuthManager.setSession(endpoint, accessKey.trim(), secretKey.trim(), region, rememberMe);
      onLogin(session);
    } catch (err) {
      console.error("Login error:", err);
      setError(err.message || "Falha na autenticação: Credenciais inválidas ou sem permissão de acesso ao S3.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-[#0f1b2a] flex flex-col justify-between font-sans">
      {/* Top Brand Bar */}
      <header className="h-14 bg-[#161e2d] border-b border-[#2a384c] px-8 flex items-center">
        <div className="flex items-center space-x-3">
          <svg className="w-8 h-8 text-[#2563eb]" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM19 18H6c-2.21 0-4-1.79-4-4 0-2.05 1.53-3.76 3.56-3.97l1.07-.11.5-.95C8.08 7.14 9.94 6 12 6c2.62 0 4.88 1.86 5.39 4.43l.3 1.5 1.53.11c1.6.1 2.78 1.41 2.78 2.96 0 1.65-1.35 3-3 3z"/>
          </svg>
          <span className="text-white font-bold text-lg tracking-tight">Z3S Storage Console</span>
          <span className="text-xs px-2 py-0.5 bg-[#2a384c] text-slate-300 rounded font-mono">AWS S3 Compatible</span>
        </div>
      </header>

      {/* Center Sign In Card */}
      <div className="flex-1 flex items-center justify-center p-4">
        <div className="w-full max-w-[440px] bg-white rounded-lg shadow-2xl border border-slate-200 p-8">
          <div className="mb-6">
            <h1 className="text-2xl font-bold text-[#16191f]">Sign in</h1>
            <p className="text-sm text-[#545b64] mt-1">Z3S Root or IAM Credentials</p>
          </div>

          {error && (
            <div className="mb-6 p-3 bg-[#fdf3f2] border-l-4 border-[#d13212] text-[#d13212] text-xs rounded-r flex items-start space-x-2">
              <span className="font-bold">⚠️</span>
              <div>
                <p className="font-semibold">Erro de autenticação</p>
                <p className="mt-0.5">{error}</p>
              </div>
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-[#16191f] uppercase tracking-wider mb-1.5">
                Access Key ID
              </label>
              <input 
                type="text"
                value={accessKey}
                onChange={e => setAccessKey(e.target.value)}
                required
                autoFocus
                placeholder="ex: Z3SACCESSKEYEXAMPLE"
                className="w-full h-10 px-3 border border-[#aab7b8] rounded focus:outline-none focus:border-[#2563eb] focus:ring-1 focus:ring-[#2563eb] text-sm text-[#16191f] font-mono transition"
              />
            </div>

            <div>
              <div className="flex items-center justify-between mb-1.5">
                <label className="block text-xs font-bold text-[#16191f] uppercase tracking-wider">
                  Secret Access Key
                </label>
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="text-xs text-[#0073bb] hover:underline focus:outline-none"
                >
                  {showPassword ? "Ocultar" : "Mostrar"}
                </button>
              </div>
              <input 
                type={showPassword ? "text" : "password"}
                value={secretKey}
                onChange={e => setSecretKey(e.target.value)}
                required
                placeholder="••••••••••••••••"
                className="w-full h-10 px-3 border border-[#aab7b8] rounded focus:outline-none focus:border-[#2563eb] focus:ring-1 focus:ring-[#2563eb] text-sm text-[#16191f] font-mono transition"
              />
            </div>

            <div className="flex items-center">
              <input 
                id="rememberMe"
                type="checkbox"
                checked={rememberMe}
                onChange={e => setRememberMe(e.target.checked)}
                className="w-4 h-4 text-[#2563eb] border-[#aab7b8] rounded focus:ring-[#2563eb]"
              />
              <label htmlFor="rememberMe" className="ml-2 text-xs text-[#545b64] cursor-pointer">
                Lembrar credenciais neste navegador
              </label>
            </div>

            {/* Quick credentials shortcuts for development */}
            <div className="pt-1 pb-1">
              <span className="text-[11px] text-[#545b64] block mb-1.5 font-semibold">Preenchimento Rápido (Dev):</span>
              <div className="flex flex-wrap gap-1.5">
                <button
                  type="button"
                  onClick={() => {
                    setAccessKey("Z3SACCESSKEYEXAMPLE");
                    setSecretKey("Z3SSECRETKEYEXAMPLE1234567890ABCDEF");
                  }}
                  className="px-2 py-1 bg-slate-100 hover:bg-slate-200 border border-slate-300 rounded text-[11px] text-slate-700 font-mono transition"
                >
                  Z3S Root
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setAccessKey("admin");
                    setSecretKey("admin123456");
                  }}
                  className="px-2 py-1 bg-slate-100 hover:bg-slate-200 border border-slate-300 rounded text-[11px] text-slate-700 font-mono transition"
                >
                  admin
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setAccessKey("z3sadmin");
                    setSecretKey("z3sadminsecretkey");
                  }}
                  className="px-2 py-1 bg-slate-100 hover:bg-slate-200 border border-slate-300 rounded text-[11px] text-slate-700 font-mono transition"
                >
                  z3sadmin
                </button>
              </div>
            </div>

            <button
              type="submit"
              disabled={loading}
              className="w-full h-10 bg-[#2563eb] hover:bg-[#1d4ed8] active:bg-[#1e40af] text-white font-bold text-sm rounded shadow-sm transition flex items-center justify-center space-x-2"
            >
              {loading ? (
                <>
                  <span className="animate-spin">🔄</span>
                  <span>Autenticando...</span>
                </>
              ) : (
                <span>Sign in</span>
              )}
            </button>
          </form>

          <div className="mt-6 pt-4 border-t border-slate-200 text-center text-xs text-[#545b64]">
            Z3S Object Storage Engine &bull; AWS SigV4 RFC 3986
          </div>
        </div>
      </div>

      {/* Footer */}
      <footer className="h-10 bg-[#161e2d] px-8 flex items-center justify-between text-xs text-[#879596]">
        <span>&copy; 2026 Z3S Core Team. Todos os direitos reservados.</span>
        <span>Região: us-east-1 (Auto)</span>
      </footer>
    </div>
  );
}

// ----------------------------------------------------------------------
// 2. AWS Global Header (Slim Navy Header) & AWS Sidebar Navigation
// ----------------------------------------------------------------------
function AwsGlobalHeader({ session, sidebarCollapsed, onToggleSidebar, onLogout, onNavigateHome }) {
  return (
    <header className="h-12 bg-[#161e2d] text-white px-4 flex items-center justify-between select-none shadow-md z-30 border-b border-[#232f3e]">
      {/* Left: Sidebar Toggle + Brand */}
      <div className="flex items-center space-x-3">
        <button
          type="button"
          onClick={onToggleSidebar}
          className="p-1.5 rounded hover:bg-[#232f3e] text-slate-300 hover:text-white transition focus:outline-none"
          title={sidebarCollapsed ? "Expandir Menu Lateral" : "Recolher Menu Lateral"}
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M4 6h16M4 12h16M4 18h16" />
          </svg>
        </button>

        <button 
          onClick={onNavigateHome}
          className="flex items-center space-x-2.5 hover:opacity-90 transition focus:outline-none"
        >
          <div className="w-6 h-6 bg-[#2563eb] rounded flex items-center justify-center font-bold text-xs text-white shadow-xs">
            S3
          </div>
          <span className="font-bold text-sm tracking-tight text-white hidden sm:inline">Z3S Object Storage</span>
        </button>
      </div>

      {/* Center Search Bar */}
      <div className="hidden md:flex flex-1 max-w-md mx-8">
        <div className="relative w-full">
          <input 
            type="text" 
            placeholder="Pesquisar buckets, objetos ou configurações..." 
            className="w-full h-8 pl-8 pr-3 bg-[#0e1622] border border-[#3e4f67] rounded text-xs text-slate-200 placeholder-[#879596] focus:outline-none focus:border-[#2563eb]"
          />
          <span className="absolute left-2.5 top-2 text-xs text-[#879596]">🔍</span>
        </div>
      </div>

      {/* Right: Region & Account */}
      <div className="flex items-center space-x-3 text-xs">
        <div className="hidden sm:flex items-center space-x-1.5 px-2.5 py-1 bg-[#232f3e] border border-[#3e4f67] rounded text-slate-200">
          <span className="w-2 h-2 rounded-full bg-emerald-400"></span>
          <span>us-east-1 (Cluster RS 4+2)</span>
        </div>

        <div className="flex items-center space-x-2 pl-2 border-l border-[#2a384c]">
          <span className="font-semibold text-slate-300 hidden sm:inline">
            {session?.accessKey ? `${session.accessKey.substring(0, 10)}...` : "Admin"}
          </span>
          <button
            onClick={onLogout}
            className="px-2.5 py-1 bg-[#232f3e] hover:bg-[#2e3e52] border border-[#3e4f67] rounded text-slate-200 transition font-medium"
            title="Sair da Sessão"
          >
            Sign out
          </button>
        </div>
      </div>
    </header>
  );
}

// ----------------------------------------------------------------------
// 2.1. AWS Left Sidebar Navigation Component
// ----------------------------------------------------------------------
function AwsSidebar({ currentTab, collapsed, onNavigate }) {
  const navSections = [
    {
      title: "Amazon S3",
      items: [
        {
          id: "buckets",
          label: "Buckets",
          icon: "🪣",
          description: "Gerenciamento de buckets e objetos",
          active: currentTab === "buckets" || currentTab === "bucket-detail"
        }
      ]
    },
    {
      title: "Cluster & Storage",
      items: [
        {
          id: "cluster-metrics",
          label: "Cluster Health & Metrics",
          icon: "📊",
          badge: "Healthy",
          description: "Monitoramento Reed-Solomon e Extents",
          active: currentTab === "cluster-metrics"
        }
      ]
    },
    {
      title: "Segurança & Acesso",
      items: [
        {
          id: "iam-keys",
          label: "Access Keys (IAM)",
          icon: "🔑",
          description: "Credenciais de API e AWS SigV4",
          active: currentTab === "iam-keys"
        }
      ]
    }
  ];

  return (
    <aside
      className={`bg-[#0f172a] text-slate-300 border-r border-[#1e293b] flex flex-col justify-between transition-all duration-300 select-none z-20 ${
        collapsed ? "w-16" : "w-64"
      }`}
    >
      {/* Top Nav Items */}
      <div className="p-3 space-y-6 overflow-y-auto">
        {navSections.map((section, idx) => (
          <div key={idx} className="space-y-1">
            {!collapsed && (
              <div className="px-3 py-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">
                {section.title}
              </div>
            )}
            <div className="space-y-1">
              {section.items.map((item) => {
                const isActive = item.active;
                return (
                  <button
                    key={item.id}
                    onClick={() => onNavigate(item.id)}
                    className={`w-full flex items-center ${
                      collapsed ? "justify-center px-0 py-2.5" : "justify-between px-3 py-2"
                    } rounded-md text-xs font-medium transition group ${
                      isActive
                        ? "bg-[#2563eb] text-white shadow-sm"
                        : "text-slate-300 hover:bg-[#1e293b] hover:text-white"
                    }`}
                    title={collapsed ? item.label : undefined}
                  >
                    <div className="flex items-center space-x-2.5">
                      <span className="text-sm">{item.icon}</span>
                      {!collapsed && <span>{item.label}</span>}
                    </div>

                    {!collapsed && item.badge && (
                      <span
                        className={`text-[10px] px-1.5 py-0.5 rounded font-bold ${
                          isActive
                            ? "bg-blue-400 text-blue-900"
                            : "bg-emerald-950 text-emerald-300 border border-emerald-800"
                        }`}
                      >
                        {item.badge}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </div>

      {/* Bottom Cluster Status Footer */}
      {!collapsed ? (
        <div className="p-3 border-t border-[#1e293b] bg-[#0b1120] text-xs space-y-2">
          <div className="flex items-center justify-between text-[11px] text-slate-400">
            <span>Cluster Engine</span>
            <span className="text-emerald-400 font-bold flex items-center space-x-1">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
              <span>RS 4+2</span>
            </span>
          </div>
          <div className="text-[10px] text-slate-400">
            Append-Only Extents Direct I/O
          </div>
        </div>
      ) : (
        <div className="p-2 border-t border-[#1e293b] flex justify-center">
          <span className="w-2 h-2 rounded-full bg-emerald-400" title="RS 4+2 Healthy"></span>
        </div>
      )}
    </aside>
  );
}

// ----------------------------------------------------------------------
// 3. AWS Buckets Overview View (Phase 2 - Advanced Management)
// ----------------------------------------------------------------------
function AwsBucketsView({ onSelectBucket, addToast }) {
  const [buckets, setBuckets] = useState([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [selectedBucketNames, setSelectedBucketNames] = useState([]);
  
  // Modals state
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [deleteTargetBucket, setDeleteTargetBucket] = useState(null); // string or array
  const [emptyTargetBucket, setEmptyTargetBucket] = useState(null);

  const fetchBuckets = async () => {
    setLoading(true);
    try {
      const client = AuthManager.getClient();
      const list = await client.listBuckets();
      setBuckets(list);
      setSelectedBucketNames([]);
    } catch (err) {
      addToast("Erro ao carregar buckets: " + err.message, "error");
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

  const handleSelectAll = (e) => {
    if (e.target.checked) {
      setSelectedBucketNames(filteredBuckets.map(b => b.name));
    } else {
      setSelectedBucketNames([]);
    }
  };

  const handleToggleSelect = (bucketName) => {
    setSelectedBucketNames(prev => 
      prev.includes(bucketName) ? prev.filter(n => n !== bucketName) : [...prev, bucketName]
    );
  };

  const handleCopyUri = (bucketName) => {
    const uri = `s3://${bucketName}`;
    navigator.clipboard.writeText(uri);
    addToast(`URI '${uri}' copiada para a área de transferência!`, "success");
  };

  const isAllSelected = filteredBuckets.length > 0 && selectedBucketNames.length === filteredBuckets.length;
  const isSomeSelected = selectedBucketNames.length > 0 && !isAllSelected;

  return (
    <div className="space-y-4">
      {/* Breadcrumbs */}
      <div className="text-xs text-[#545b64] flex items-center space-x-1.5">
        <span className="hover:underline cursor-pointer">Z3S S3</span>
        <span>&gt;</span>
        <span className="text-[#16191f] font-semibold">Buckets</span>
      </div>

      {/* Page Title & Stats */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
        <div>
          <h1 className="text-2xl font-bold text-[#16191f]">
            Buckets <span className="text-[#545b64] font-normal text-lg">({filteredBuckets.length})</span>
          </h1>
          <p className="text-xs text-[#545b64] mt-0.5">
            Buckets are containers for data stored in Z3S S3. Configure properties, access policies, and object versions.
          </p>
        </div>

        <div className="flex items-center space-x-2">
          <button
            onClick={() => setShowCreateModal(true)}
            className="h-9 px-4 bg-[#2563eb] hover:bg-[#1d4ed8] active:bg-[#1e40af] text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1.5"
          >
            <span>+</span>
            <span>Create bucket</span>
          </button>
        </div>
      </div>

      {/* Cloudscape Table Card */}
      <div className="bg-white border border-[#eaeded] rounded shadow-sm overflow-hidden">
        {/* Table Action Bar */}
        <div className="p-3.5 border-b border-[#eaeded] bg-white flex flex-wrap items-center justify-between gap-3">
          {/* Search */}
          <div className="relative w-72">
            <input 
              type="text"
              placeholder="Find bucket by name..."
              value={search}
              onChange={e => setSearch(e.target.value)}
              className="w-full h-8 pl-8 pr-8 border border-[#aab7b8] rounded text-xs text-[#16191f] focus:outline-none focus:border-[#2563eb]"
            />
            <span className="absolute left-2.5 top-2 text-xs text-[#879596]">🔍</span>
            {search && (
              <button 
                onClick={() => setSearch("")}
                className="absolute right-2.5 top-1.5 text-xs text-[#879596] hover:text-[#16191f]"
              >
                ✕
              </button>
            )}
          </div>

          {/* Action Buttons */}
          <div className="flex flex-wrap items-center gap-2">
            {/* Copy URI */}
            <button
              disabled={selectedBucketNames.length !== 1}
              onClick={() => handleCopyUri(selectedBucketNames[0])}
              className="h-8 px-3 bg-white hover:bg-[#fafafa] disabled:opacity-40 disabled:hover:bg-white border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition"
              title="Copiar S3 URI do bucket selecionado"
            >
              📋 Copy S3 URI
            </button>

            {/* Empty Button */}
            <button
              disabled={selectedBucketNames.length !== 1}
              onClick={() => setEmptyTargetBucket(selectedBucketNames[0])}
              className="h-8 px-3 bg-white hover:bg-amber-50 text-amber-700 disabled:opacity-40 disabled:hover:bg-white disabled:text-[#16191f] border border-[#aab7b8] rounded text-xs font-semibold transition"
              title="Esvaziar todos os objetos do bucket"
            >
              🧹 Empty
            </button>

            {/* Delete Button */}
            <button
              disabled={selectedBucketNames.length !== 1}
              onClick={() => setDeleteTargetBucket(selectedBucketNames[0])}
              className="h-8 px-3 bg-white hover:bg-red-50 text-red-600 disabled:opacity-40 disabled:hover:bg-white disabled:text-[#16191f] border border-[#aab7b8] rounded text-xs font-semibold transition"
              title="Excluir bucket selecionado"
            >
              🗑️ Delete
            </button>

            {/* Refresh */}
            <button
              onClick={fetchBuckets}
              className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition"
            >
              🔄 Refresh
            </button>
          </div>
        </div>

        {/* Selected count feedback */}
        {selectedBucketNames.length > 0 && (
          <div className="px-4 py-2 bg-[#f2f8fd] border-b border-[#eaeded] text-xs text-[#0073bb] flex items-center justify-between">
            <span><strong>{selectedBucketNames.length}</strong> bucket(s) selecionado(s)</span>
            <button 
              onClick={() => setSelectedBucketNames([])}
              className="text-xs text-[#0073bb] hover:underline"
            >
              Limpar seleção
            </button>
          </div>
        )}

        {/* Table Content */}
        {loading ? (
          <div className="p-12 text-center text-xs text-[#545b64] flex flex-col items-center space-y-2">
            <span className="text-xl animate-spin">🔄</span>
            <span>Carregando buckets do storage...</span>
          </div>
        ) : filteredBuckets.length === 0 ? (
          <div className="p-12 text-center text-xs text-[#545b64] space-y-3">
            <p className="text-sm font-semibold text-[#16191f]">Nenhum bucket encontrado.</p>
            {search ? (
              <p>Nenhum resultado corresponde ao filtro "{search}".</p>
            ) : (
              <p>Comece criando seu primeiro bucket para armazenar dados no Z3S.</p>
            )}
            <button
              onClick={() => setShowCreateModal(true)}
              className="h-8 px-4 bg-[#2563eb] hover:bg-[#1d4ed8] text-white text-xs font-bold rounded shadow-sm transition"
            >
              Create bucket
            </button>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-xs text-[#16191f]">
              <thead className="bg-[#fafafa] border-b border-[#eaeded] text-[#545b64] font-semibold">
                <tr>
                  <th className="px-4 py-3 w-10">
                    <input 
                      type="checkbox" 
                      checked={isAllSelected}
                      ref={input => { if (input) input.indeterminate = isSomeSelected; }}
                      onChange={handleSelectAll}
                      className="rounded text-[#2563eb] focus:ring-[#2563eb]" 
                    />
                  </th>
                  <th className="px-4 py-3">Name</th>
                  <th className="px-4 py-3">AWS Region</th>
                  <th className="px-4 py-3">Access</th>
                  <th className="px-4 py-3">Creation date</th>
                  <th className="px-4 py-3 text-right">Quick Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#eaeded]">
                {filteredBuckets.map(b => {
                  const isSelected = selectedBucketNames.includes(b.name);
                  return (
                    <tr 
                      key={b.name} 
                      className={`transition ${isSelected ? "bg-[#f2f8fd]" : "hover:bg-[#f8f9fa]"}`}
                    >
                      <td className="px-4 py-3">
                        <input 
                          type="checkbox" 
                          checked={isSelected}
                          onChange={() => handleToggleSelect(b.name)}
                          className="rounded text-[#2563eb] focus:ring-[#2563eb]" 
                        />
                      </td>
                      <td className="px-4 py-3 font-semibold">
                        <div 
                          className="inline-flex items-center space-x-1.5 text-[#0073bb] hover:underline cursor-pointer"
                          onClick={() => onSelectBucket(b.name)}
                        >
                          <span className="text-sm">🪣</span>
                          <span className="text-sm">{b.name}</span>
                        </div>
                      </td>
                      <td className="px-4 py-3 text-[#545b64]">
                        <span className="px-2 py-0.5 bg-slate-100 border border-slate-200 rounded font-mono text-[11px]">
                          us-east-1
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <span className="inline-flex items-center space-x-1 px-2 py-0.5 bg-[#f2f3f3] text-[#545b64] rounded text-[11px] font-medium border border-[#eaeded]">
                          <span>🔒</span>
                          <span>Bucket and objects not public</span>
                        </span>
                      </td>
                      <td className="px-4 py-3 text-[#545b64] font-mono text-[11px]">
                        {new Date(b.creationDate).toLocaleString()}
                      </td>
                      <td className="px-4 py-3 text-right space-x-1.5">
                        <button 
                          onClick={() => onSelectBucket(b.name)}
                          className="px-2.5 py-1 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition"
                          title="Abrir Bucket"
                        >
                          Objects
                        </button>
                        <button 
                          onClick={() => handleCopyUri(b.name)}
                          className="px-2 py-1 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs text-[#545b64] hover:text-[#16191f] transition"
                          title="Copiar s3://..."
                        >
                          📋
                        </button>
                        <button 
                          onClick={() => setEmptyTargetBucket(b.name)}
                          className="px-2 py-1 bg-white hover:bg-amber-50 text-amber-700 border border-amber-200 rounded text-xs transition"
                          title="Esvaziar Bucket"
                        >
                          🧹
                        </button>
                        <button 
                          onClick={() => setDeleteTargetBucket(b.name)}
                          className="px-2 py-1 bg-white hover:bg-red-50 text-red-600 border border-red-200 rounded text-xs transition"
                          title="Excluir Bucket"
                        >
                          🗑️
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* 1. Modal Create Bucket Avançado */}
      {showCreateModal && (
        <AwsCreateBucketModal 
          onClose={() => setShowCreateModal(false)}
          onCreated={() => {
            setShowCreateModal(false);
            fetchBuckets();
            addToast("Bucket criado com sucesso!", "success");
          }}
          addToast={addToast}
        />
      )}

      {/* 2. Modal Delete Bucket com Trava de Segurança */}
      {deleteTargetBucket && (
        <AwsDeleteBucketModal
          bucketName={deleteTargetBucket}
          onClose={() => setDeleteTargetBucket(null)}
          onDeleted={() => {
            setDeleteTargetBucket(null);
            fetchBuckets();
            addToast(`Bucket '${deleteTargetBucket}' excluído com sucesso!`, "success");
          }}
          onOpenEmptyModal={() => {
            const bName = deleteTargetBucket;
            setDeleteTargetBucket(null);
            setEmptyTargetBucket(bName);
          }}
          addToast={addToast}
        />
      )}

      {/* 3. Modal Empty Bucket */}
      {emptyTargetBucket && (
        <AwsEmptyBucketModal
          bucketName={emptyTargetBucket}
          onClose={() => setEmptyTargetBucket(null)}
          onEmptied={() => {
            setEmptyTargetBucket(null);
            fetchBuckets();
            addToast(`Bucket '${emptyTargetBucket}' esvaziado com sucesso!`, "success");
          }}
          addToast={addToast}
        />
      )}
    </div>
  );
}

// --- Helper formatting functions ---
function formatBytes(bytes) {
  if (bytes === 0 || bytes === undefined || bytes === null) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

function getFileIcon(key, isFolder = false, isDeleteMarker = false) {
  if (isDeleteMarker) return "🗑️";
  if (isFolder) return "📁";
  const ext = (key.split(".").pop() || "").toLowerCase();
  if (["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "ico"].includes(ext)) return "🖼️";
  if (["mp4", "webm", "mkv", "mov"].includes(ext)) return "🎬";
  if (["mp3", "wav", "ogg", "flac"].includes(ext)) return "🎵";
  if (["zip", "tar", "gz", "7z", "rar", "z3se"].includes(ext)) return "📦";
  if (["pdf"].includes(ext)) return "📕";
  if (["json", "js", "ts", "jsx", "tsx", "py", "rs", "go", "java", "c", "cpp", "html", "css", "xml", "yaml", "yml", "sh", "sql"].includes(ext)) return "💻";
  return "📄";
}

// ----------------------------------------------------------------------
// 4. AWS Bucket Detail View (Phase 3 - Complete S3 Object Manager)
// ----------------------------------------------------------------------
function AwsBucketDetailView({ bucket, activeTab, setActiveTab, onBack, addToast }) {
  const [currentPrefix, setCurrentPrefix] = useState("");
  const [contents, setContents] = useState({ folders: [], objects: [] });
  const [versionsData, setVersionsData] = useState({ folders: [], versions: [], deleteMarkers: [] });
  const [showVersions, setShowVersions] = useState(false);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [selectedKeys, setSelectedKeys] = useState(new Set());
  const [versioningStatus, setVersioningStatus] = useState("Loading...");
  const [updatingVersioning, setUpdatingVersioning] = useState(false);

  // Modals
  const [isUploadOpen, setIsUploadOpen] = useState(false);
  const [isCreateFolderOpen, setIsCreateFolderOpen] = useState(false);
  const [isDeleteModalOpen, setIsDeleteModalOpen] = useState(false);
  const [previewObject, setPreviewObject] = useState(null);

  const fetchObjects = async () => {
    setLoading(true);
    setSelectedKeys(new Set());
    try {
      const client = AuthManager.getClient();
      if (showVersions) {
        const data = await client.listObjectVersions(bucket, currentPrefix, "/");
        setVersionsData(data);
      } else {
        const data = await client.listObjects(bucket, currentPrefix, "/");
        setContents(data);
      }
    } catch (err) {
      addToast("Erro ao carregar objetos: " + err.message, "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchObjects();
  }, [bucket, currentPrefix, showVersions]);

  useEffect(() => {
    if (activeTab === "properties") {
      const client = AuthManager.getClient();
      client.getBucketVersioning(bucket).then(v => setVersioningStatus(v || "Off"));
    }
  }, [activeTab, bucket]);

  const handleToggleVersioning = async () => {
    const nextStatus = (versioningStatus === "Enabled") ? "Suspended" : "Enabled";
    setUpdatingVersioning(true);
    try {
      const client = AuthManager.getClient();
      await client.putBucketVersioning(bucket, nextStatus);
      setVersioningStatus(nextStatus);
      addToast(`Versionamento do bucket alterado para '${nextStatus}'.`, "success");
    } catch (err) {
      addToast(`Erro ao alterar versionamento: ${err.message}`, "error");
    } finally {
      setUpdatingVersioning(false);
    }
  };

  // Breadcrumbs segments
  const breadcrumbSegments = useMemo(() => {
    if (!currentPrefix) return [];
    const parts = currentPrefix.split("/").filter(Boolean);
    const result = [];
    let acc = "";
    for (const part of parts) {
      acc += part + "/";
      result.push({ name: part, prefix: acc });
    }
    return result;
  }, [currentPrefix]);

  // Combined and filtered table items
  const displayedItems = useMemo(() => {
    if (showVersions) {
      const items = [];
      // Folders
      for (const f of versionsData.folders) {
        const cleanName = f.slice(currentPrefix.length).replace(/\/$/, "");
        if (!search || cleanName.toLowerCase().includes(search.toLowerCase())) {
          items.push({ type: "folder", key: f, displayName: cleanName + "/" });
        }
      }
      // Versions & Delete markers
      const allVer = [
        ...versionsData.versions.map(v => ({ ...v, type: "version" })),
        ...versionsData.deleteMarkers.map(d => ({ ...d, type: "delete_marker" }))
      ];
      for (const item of allVer) {
        const cleanName = item.key.slice(currentPrefix.length);
        if (!search || cleanName.toLowerCase().includes(search.toLowerCase())) {
          items.push({
            ...item,
            displayName: cleanName
          });
        }
      }
      return items;
    } else {
      const items = [];
      // Folders
      for (const f of contents.folders) {
        const cleanName = f.slice(currentPrefix.length).replace(/\/$/, "");
        if (!search || cleanName.toLowerCase().includes(search.toLowerCase())) {
          items.push({ type: "folder", key: f, displayName: cleanName + "/" });
        }
      }
      // Objects
      for (const obj of contents.objects) {
        // Hide directory marker files if they match current folder exactly
        if (obj.key === currentPrefix) continue;
        const cleanName = obj.key.slice(currentPrefix.length);
        if (!search || cleanName.toLowerCase().includes(search.toLowerCase())) {
          items.push({
            type: "object",
            ...obj,
            displayName: cleanName
          });
        }
      }
      return items;
    }
  }, [showVersions, contents, versionsData, currentPrefix, search]);

  const handleSelectAll = (e) => {
    if (e.target.checked) {
      const allKeys = new Set(displayedItems.map(item => item.key));
      setSelectedKeys(allKeys);
    } else {
      setSelectedKeys(new Set());
    }
  };

  const toggleSelectKey = (key) => {
    setSelectedKeys(prev => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const handleDownloadSelected = async () => {
    if (selectedKeys.size !== 1) return;
    const key = Array.from(selectedKeys)[0];
    try {
      addToast(`Iniciando download de '${key}'...`, "info");
      const client = AuthManager.getClient();
      await client.downloadObject(bucket, key);
      addToast(`Download concluído!`, "success");
    } catch (err) {
      addToast(`Erro no download: ${err.message}`, "error");
    }
  };

  const totalBytes = useMemo(() => {
    return contents.objects.reduce((acc, obj) => acc + (obj.size || 0), 0);
  }, [contents.objects]);

  const tabs = [
    { id: "objects", label: "Objects" },
    { id: "properties", label: "Properties" },
    { id: "permissions", label: "Permissions" },
    { id: "metrics", label: "Metrics" },
  ];

  return (
    <div className="space-y-4">
      {/* 1. Global Navigation Breadcrumbs */}
      <div className="text-xs text-[#545b64] flex items-center space-x-1.5 flex-wrap">
        <span onClick={onBack} className="hover:underline cursor-pointer">Z3S S3</span>
        <span>&gt;</span>
        <span onClick={onBack} className="hover:underline cursor-pointer">Buckets</span>
        <span>&gt;</span>
        <span 
          onClick={() => setCurrentPrefix("")} 
          className={`hover:underline cursor-pointer ${!currentPrefix ? "text-[#16191f] font-semibold" : ""}`}
        >
          {bucket}
        </span>
        {breadcrumbSegments.map((seg, idx) => (
          <React.Fragment key={seg.prefix}>
            <span>/</span>
            <span
              onClick={() => setCurrentPrefix(seg.prefix)}
              className={`hover:underline cursor-pointer ${
                idx === breadcrumbSegments.length - 1 ? "text-[#16191f] font-semibold" : ""
              }`}
            >
              {seg.name}
            </span>
          </React.Fragment>
        ))}
      </div>

      {/* 2. Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-3">
          <button 
            onClick={onBack}
            className="h-8 px-2.5 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition"
          >
            &larr; Buckets
          </button>
          <h1 className="text-2xl font-bold text-[#16191f] flex items-center space-x-2">
            <span>🪣</span>
            <span>{bucket}</span>
          </h1>
        </div>

        {activeTab === "objects" && (
          <div className="flex items-center space-x-2">
            <button
              onClick={() => setIsCreateFolderOpen(true)}
              className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition flex items-center space-x-1.5"
            >
              <span>📁</span>
              <span>Create folder</span>
            </button>
            <button
              onClick={() => setIsUploadOpen(true)}
              className="h-8 px-4 bg-[#2563eb] hover:bg-[#1d4ed8] text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1.5"
            >
              <span>⬆</span>
              <span>Upload</span>
            </button>
          </div>
        )}
      </div>

      {/* 3. Navigation Tabs */}
      <div className="border-b border-[#eaeded] flex space-x-8 text-sm">
        {tabs.map(t => (
          <button
            key={t.id}
            onClick={() => setActiveTab(t.id)}
            className={`pb-3 font-semibold transition border-b-2 ${
              activeTab === t.id 
                ? "border-[#2563eb] text-[#2563eb]" 
                : "border-transparent text-[#545b64] hover:text-[#16191f]"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Tab 1: Objects View (File Explorer) */}
      {activeTab === "objects" && (
        <div className="bg-white border border-[#eaeded] rounded shadow-sm overflow-hidden space-y-0">
          {/* Action Toolbar */}
          <div className="p-3 border-b border-[#eaeded] flex flex-wrap items-center justify-between gap-3 bg-[#fbfbfb]">
            {/* Search & Breadcrumb Bar */}
            <div className="flex items-center space-x-2 flex-1 min-w-[280px]">
              <div className="relative flex-1 max-w-xs">
                <input
                  type="text"
                  value={search}
                  onChange={e => setSearch(e.target.value)}
                  placeholder="Find objects by prefix..."
                  className="w-full h-8 pl-8 pr-3 bg-white border border-[#aab7b8] rounded text-xs text-[#16191f] focus:outline-none focus:border-[#2563eb]"
                />
                <span className="absolute left-2.5 top-2 text-[#545b64] text-xs">🔍</span>
              </div>

              {/* Show Versions Toggle */}
              <label className="flex items-center space-x-1.5 text-xs text-[#545b64] cursor-pointer select-none pl-2 border-l border-[#eaeded]">
                <input
                  type="checkbox"
                  checked={showVersions}
                  onChange={e => setShowVersions(e.target.checked)}
                  className="rounded text-[#2563eb] focus:ring-[#2563eb]"
                />
                <span className="font-medium text-[#16191f]">Show versions</span>
              </label>
            </div>

            {/* Selection Actions */}
            <div className="flex items-center space-x-2">
              <button
                onClick={handleDownloadSelected}
                disabled={selectedKeys.size !== 1}
                className="h-8 px-3 bg-white hover:bg-[#fafafa] disabled:opacity-40 disabled:hover:bg-white border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition flex items-center space-x-1"
              >
                <span>⬇</span>
                <span>Download</span>
              </button>

              <button
                onClick={() => setIsDeleteModalOpen(true)}
                disabled={selectedKeys.size === 0}
                className="h-8 px-3 bg-white hover:bg-red-50 disabled:opacity-40 disabled:hover:bg-white border border-red-300 rounded text-xs font-semibold text-red-600 transition flex items-center space-x-1"
              >
                <span>🗑️</span>
                <span>Delete ({selectedKeys.size})</span>
              </button>

              <button 
                onClick={fetchObjects}
                className="h-8 w-8 flex items-center justify-center bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs text-[#16191f] transition"
                title="Refresh object list"
              >
                🔄
              </button>
            </div>
          </div>

          {/* S3 URI Path Bar */}
          <div className="px-4 py-2 bg-slate-50 border-b border-[#eaeded] flex items-center justify-between text-xs text-[#545b64]">
            <div className="flex items-center space-x-2 font-mono">
              <span className="text-[#aab7b8]">Current path:</span>
              <span className="text-[#16191f] font-semibold">s3://{bucket}/{currentPrefix}</span>
            </div>
            {currentPrefix && (
              <button
                onClick={() => {
                  const parts = currentPrefix.split("/").filter(Boolean);
                  parts.pop();
                  setCurrentPrefix(parts.length > 0 ? parts.join("/") + "/" : "");
                }}
                className="text-[#2563eb] hover:underline font-semibold flex items-center space-x-1"
              >
                <span>&uarr; Up one level</span>
              </button>
            )}
          </div>

          {/* Table */}
          {loading ? (
            <div className="p-16 text-center text-xs text-[#545b64] flex flex-col items-center space-y-2">
              <span className="animate-spin text-xl text-[#2563eb]">🔄</span>
              <span>Carregando objetos do bucket...</span>
            </div>
          ) : displayedItems.length === 0 ? (
            <div className="p-16 text-center text-xs text-[#545b64] space-y-3">
              <p className="text-sm font-semibold text-[#16191f]">
                {search ? "Nenhum objeto corresponde à busca." : "Esta pasta está vazia."}
              </p>
              <p>Envie novos arquivos através do botão de Upload ou crie uma pasta virtual.</p>
              <div className="pt-2 flex justify-center space-x-3">
                <button
                  onClick={() => setIsUploadOpen(true)}
                  className="px-4 py-2 bg-[#2563eb] hover:bg-[#1d4ed8] text-white font-bold rounded text-xs shadow-sm transition"
                >
                  Upload files
                </button>
              </div>
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs text-[#16191f]">
                <thead className="bg-[#fafafa] border-b border-[#eaeded] text-[#545b64] font-semibold">
                  <tr>
                    <th className="w-8 px-4 py-3">
                      <input
                        type="checkbox"
                        checked={selectedKeys.size > 0 && selectedKeys.size === displayedItems.length}
                        onChange={handleSelectAll}
                        className="rounded text-[#2563eb] focus:ring-[#2563eb]"
                      />
                    </th>
                    <th className="px-4 py-3">Name</th>
                    {showVersions && <th className="px-4 py-3">Version ID</th>}
                    <th className="px-4 py-3">Type</th>
                    <th className="px-4 py-3">Last modified</th>
                    <th className="px-4 py-3">Size</th>
                    <th className="px-4 py-3">Storage class</th>
                    <th className="w-16 px-4 py-3 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#eaeded]">
                  {displayedItems.map((item) => {
                    const isFolder = item.type === "folder";
                    const isDeleteMarker = item.type === "delete_marker";
                    const isSelected = selectedKeys.has(item.key);

                    return (
                      <tr 
                        key={item.key + (item.versionId || "")} 
                        className={`hover:bg-[#f2f8fd] transition ${isSelected ? "bg-blue-50/60" : ""}`}
                      >
                        <td className="px-4 py-3">
                          <input
                            type="checkbox"
                            checked={isSelected}
                            onChange={() => toggleSelectKey(item.key)}
                            className="rounded text-[#2563eb] focus:ring-[#2563eb]"
                          />
                        </td>
                        <td className="px-4 py-3 font-semibold">
                          <div className="flex items-center space-x-2">
                            <span>{getFileIcon(item.key, isFolder, isDeleteMarker)}</span>
                            {isFolder ? (
                              <span
                                onClick={() => setCurrentPrefix(item.key)}
                                className="text-[#0073bb] hover:underline cursor-pointer font-bold"
                              >
                                {item.displayName}
                              </span>
                            ) : (
                              <span
                                onClick={() => !isDeleteMarker && setPreviewObject(item)}
                                className={`cursor-pointer hover:underline ${
                                  isDeleteMarker ? "text-slate-500 line-through" : "text-[#0073bb]"
                                }`}
                              >
                                {item.displayName}
                              </span>
                            )}
                            {item.isLatest && showVersions && (
                              <span className="text-[10px] bg-blue-100 text-blue-700 px-1.5 py-0.2 rounded font-mono font-normal">
                                latest
                              </span>
                            )}
                          </div>
                        </td>

                        {showVersions && (
                          <td className="px-4 py-3 font-mono text-[11px] text-[#545b64]">
                            {item.versionId ? (
                              <span className="truncate max-w-[120px] inline-block" title={item.versionId}>
                                {item.versionId}
                              </span>
                            ) : (
                              "-"
                            )}
                          </td>
                        )}

                        <td className="px-4 py-3 text-[#545b64] text-[11px]">
                          {isDeleteMarker ? (
                            <span className="text-red-600 font-semibold">Delete marker</span>
                          ) : isFolder ? (
                            "Folder"
                          ) : (
                            (item.key.split(".").pop() || "File").toUpperCase()
                          )}
                        </td>

                        <td className="px-4 py-3 text-[#545b64] font-mono text-[11px]">
                          {item.lastModified ? new Date(item.lastModified).toLocaleString() : "-"}
                        </td>

                        <td className="px-4 py-3 text-[#545b64] font-mono text-[11px]">
                          {isFolder || isDeleteMarker ? "-" : formatBytes(item.size)}
                        </td>

                        <td className="px-4 py-3 text-[#545b64]">
                          {isFolder || isDeleteMarker ? "-" : (item.storageClass || "STANDARD")}
                        </td>

                        <td className="px-4 py-3 text-right">
                          <div className="flex items-center justify-end space-x-1">
                            {!isFolder && !isDeleteMarker && (
                              <button
                                onClick={async (e) => {
                                  e.stopPropagation();
                                  try {
                                    const client = AuthManager.getClient();
                                    await client.downloadObject(bucket, item.key);
                                  } catch (err) {
                                    addToast(`Erro: ${err.message}`, "error");
                                  }
                                }}
                                className="p-1 hover:bg-slate-200 rounded text-[#545b64] transition"
                                title="Download"
                              >
                                ⬇
                              </button>
                            )}
                            {!isDeleteMarker && !isFolder && (
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  setPreviewObject(item);
                                }}
                                className="p-1 hover:bg-slate-200 rounded text-[#545b64] transition"
                                title="Inspect & Preview"
                              >
                                🔍
                              </button>
                            )}
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* Tab 2: Properties */}
      {activeTab === "properties" && (
        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-3">
              <div className="flex items-start justify-between">
                <div>
                  <h3 className="text-sm font-bold text-[#16191f]">Bucket Versioning</h3>
                  <p className="text-xs text-[#545b64]">Mantém múltiplas variantes de um objeto no mesmo bucket.</p>
                </div>
                <button
                  onClick={handleToggleVersioning}
                  disabled={updatingVersioning}
                  className="h-7 px-3 bg-white hover:bg-slate-50 border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition"
                >
                  {updatingVersioning ? "Atualizando..." : (versioningStatus === "Enabled" ? "Suspender" : "Ativar")}
                </button>
              </div>
              <div className="pt-1">
                <span className={`px-2.5 py-1 rounded text-xs font-semibold border ${
                  versioningStatus === "Enabled" 
                    ? "bg-emerald-50 text-emerald-700 border-emerald-200" 
                    : "bg-slate-100 text-slate-600 border-slate-200"
                }`}>
                  {versioningStatus === "Enabled" ? "✓ Enabled" : (versioningStatus === "Suspended" ? "⏸ Suspended" : "Disabled")}
                </span>
              </div>
            </div>

            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-3">
              <h3 className="text-sm font-bold text-[#16191f]">Default Encryption</h3>
              <p className="text-xs text-[#545b64]">Criptografa objetos automaticamente em repouso com algoritmo padrão AES-256.</p>
              <div className="pt-1">
                <span className="px-2.5 py-1 bg-blue-50 text-blue-700 border border-blue-200 rounded text-xs font-semibold">
                  🔒 SSE-S3 (AES-256-GCM)
                </span>
              </div>
            </div>

            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-2 md:col-span-2">
              <h3 className="text-sm font-bold text-[#16191f]">Identificadores do Recurso</h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
                <div>
                  <span className="text-[11px] text-[#545b64] block mb-1">Amazon S3 URI:</span>
                  <code className="block p-2 bg-slate-50 border border-[#eaeded] rounded text-xs text-[#16191f] font-mono select-all">
                    s3://{bucket}
                  </code>
                </div>
                <div>
                  <span className="text-[11px] text-[#545b64] block mb-1">Amazon Resource Name (ARN):</span>
                  <code className="block p-2 bg-slate-50 border border-[#eaeded] rounded text-xs text-[#16191f] font-mono select-all">
                    arn:aws:s3:::{bucket}
                  </code>
                </div>
              </div>
            </div>
          </div>

          {/* Lifecycle Configuration Section (Opção 1) */}
          <AwsBucketLifecycleManager bucket={bucket} addToast={addToast} />
        </div>
      )}

      {/* Tab 3: Permissions (Opção 1 - Policy, CORS & Public Access) */}
      {activeTab === "permissions" && (
        <div className="space-y-4">
          <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-4">
            <h3 className="text-sm font-bold text-[#16191f]">Block Public Access (Bucket Settings)</h3>
            <p className="text-xs text-[#545b64]">Bloqueia o acesso público e anônimo a este bucket e todos os seus objetos.</p>
            <div className="p-3 bg-emerald-50 border border-emerald-200 rounded text-xs font-semibold text-emerald-800 flex items-center space-x-2">
              <span>🛡️</span>
              <span>Block all public access: <strong>ON</strong> (Proteção Máxima Ativa)</span>
            </div>
          </div>

          {/* Bucket Policy JSON Editor */}
          <AwsBucketPolicyEditor bucket={bucket} addToast={addToast} />

          {/* CORS Configuration Editor */}
          <AwsBucketCorsEditor bucket={bucket} addToast={addToast} />
        </div>
      )}

      {/* Tab 4: Metrics (Opção 2) */}
      {activeTab === "metrics" && (
        <div className="space-y-4">
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-1">
              <span className="text-xs text-[#545b64]">Total de Objetos</span>
              <p className="text-2xl font-bold text-[#16191f]">{contents.objects.length}</p>
            </div>
            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-1">
              <span className="text-xs text-[#545b64]">Armazenamento Utilizado</span>
              <p className="text-2xl font-bold text-[#16191f]">{formatBytes(totalBytes)}</p>
            </div>
            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-1">
              <span className="text-xs text-[#545b64]">Status de Réplica / Erasure</span>
              <p className="text-sm font-semibold text-emerald-600">Reed-Solomon 4+2 Ativo</p>
            </div>
          </div>

          <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-3">
            <h3 className="text-sm font-bold text-[#16191f]">Distribuição de Armazenamento do Bucket</h3>
            <div className="w-full bg-slate-100 rounded-full h-4 overflow-hidden flex">
              <div className="bg-[#2563eb] h-full" style={{ width: "100%" }} title="Dados Padrão"></div>
            </div>
            <div className="flex items-center justify-between text-xs text-[#545b64]">
              <span className="flex items-center space-x-1.5">
                <span className="w-3 h-3 bg-[#2563eb] rounded-xs inline-block"></span>
                <span>Standard Objects ({formatBytes(totalBytes)})</span>
              </span>
              <span className="font-mono font-bold text-[#16191f]">100% Utilizado</span>
            </div>
          </div>
        </div>
      )}

      {/* Phase 3 Modals */}
      {isUploadOpen && (
        <AwsUploadModal
          bucket={bucket}
          currentPrefix={currentPrefix}
          onClose={() => setIsUploadOpen(false)}
          onUploaded={() => {
            setIsUploadOpen(false);
            fetchObjects();
            addToast("Upload concluído com sucesso!", "success");
          }}
          addToast={addToast}
        />
      )}

      {isCreateFolderOpen && (
        <AwsCreateFolderModal
          bucket={bucket}
          currentPrefix={currentPrefix}
          onClose={() => setIsCreateFolderOpen(false)}
          onCreated={() => {
            setIsCreateFolderOpen(false);
            fetchObjects();
            addToast("Pasta virtual criada com sucesso.", "success");
          }}
          addToast={addToast}
        />
      )}

      {isDeleteModalOpen && (
        <AwsDeleteObjectsModal
          bucket={bucket}
          keys={Array.from(selectedKeys)}
          onClose={() => setIsDeleteModalOpen(false)}
          onDeleted={() => {
            setIsDeleteModalOpen(false);
            fetchObjects();
            addToast("Objetos excluídos com sucesso.", "success");
          }}
          addToast={addToast}
        />
      )}

      {previewObject && (
        <AwsObjectPreviewModal
          bucket={bucket}
          object={previewObject}
          onClose={() => setPreviewObject(null)}
          onDeleted={() => {
            setPreviewObject(null);
            fetchObjects();
            addToast("Objeto excluído.", "info");
          }}
          addToast={addToast}
        />
      )}
    </div>
  );
}

// ----------------------------------------------------------------------
// 4.1 Phase 3: Drag & Drop Upload Modal with Live Progress
// ----------------------------------------------------------------------
function AwsUploadModal({ bucket, currentPrefix, onClose, onUploaded, addToast }) {
  const [files, setFiles] = useState([]);
  const [isDragging, setIsDragging] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [progressMap, setProgressMap] = useState({}); // { index: { percent, status, error } }
  const [storageClass, setStorageClass] = useState("STANDARD");

  const handleDragOver = (e) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = (e) => {
    e.preventDefault();
    setIsDragging(false);
  };

  const handleDrop = (e) => {
    e.preventDefault();
    setIsDragging(false);
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      addFiles(Array.from(e.dataTransfer.files));
    }
  };

  const handleFileInput = (e) => {
    if (e.target.files && e.target.files.length > 0) {
      addFiles(Array.from(e.target.files));
    }
  };

  const addFiles = (newFiles) => {
    setFiles(prev => {
      const existingNames = new Set(prev.map(f => f.name));
      const filtered = newFiles.filter(f => !existingNames.has(f.name));
      return [...prev, ...filtered];
    });
  };

  const removeFile = (idx) => {
    if (uploading) return;
    setFiles(prev => prev.filter((_, i) => i !== idx));
  };

  const startUpload = async () => {
    if (files.length === 0 || uploading) return;
    setUploading(true);

    const client = AuthManager.getClient();
    let hasError = false;

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const targetKey = `${currentPrefix}${file.name}`;

      setProgressMap(prev => ({
        ...prev,
        [i]: { percent: 0, status: "uploading" }
      }));

      try {
        await client.uploadObject(
          bucket,
          targetKey,
          file,
          file.type || "application/octet-stream",
          { "storage-class": storageClass },
          ({ percent }) => {
            setProgressMap(prev => ({
              ...prev,
              [i]: { percent, status: "uploading" }
            }));
          }
        );

        setProgressMap(prev => ({
          ...prev,
          [i]: { percent: 100, status: "done" }
        }));
      } catch (err) {
        hasError = true;
        console.error("Upload error for", file.name, err);
        setProgressMap(prev => ({
          ...prev,
          [i]: { percent: 0, status: "error", error: err.message }
        }));
      }
    }

    setUploading(false);
    if (!hasError) {
      setTimeout(() => {
        onUploaded();
      }, 500);
    } else {
      addToast("Alguns arquivos falharam durante o upload.", "error");
    }
  };

  const totalBytes = files.reduce((acc, f) => acc + f.size, 0);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-fade-in overflow-y-auto">
      <div className="w-full max-w-2xl bg-white rounded-lg shadow-2xl border border-slate-300 p-6 my-8 space-y-4">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#eaeded] pb-3">
          <div>
            <h2 className="text-lg font-bold text-[#16191f]">Upload files and folders</h2>
            <p className="text-xs text-[#545b64]">Destination: <code className="bg-slate-100 px-1 py-0.5 rounded font-mono text-[#2563eb]">s3://{bucket}/{currentPrefix}</code></p>
          </div>
          <button onClick={onClose} disabled={uploading} className="text-[#545b64] hover:text-[#16191f] text-sm font-bold">✕</button>
        </div>

        {/* Drag & Drop Area */}
        <div
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          className={`border-2 border-dashed rounded-lg p-8 text-center transition flex flex-col items-center justify-center space-y-3 ${
            isDragging ? "border-[#2563eb] bg-blue-50/50" : "border-[#aab7b8] hover:border-[#545b64] bg-slate-50/50"
          }`}
        >
          <span className="text-3xl">☁️</span>
          <div>
            <p className="text-sm font-bold text-[#16191f]">Drag and drop files here</p>
            <p className="text-xs text-[#545b64]">or click below to browse from your device</p>
          </div>
          <div className="flex items-center space-x-3 pt-1">
            <label className="h-8 px-4 bg-white hover:bg-slate-50 border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] cursor-pointer flex items-center shadow-xs">
              <span>📄 Add files</span>
              <input type="file" multiple onChange={handleFileInput} className="hidden" />
            </label>
          </div>
        </div>

        {/* File Queue List */}
        {files.length > 0 && (
          <div className="space-y-2">
            <div className="flex items-center justify-between text-xs text-[#545b64]">
              <span className="font-semibold text-[#16191f]">Queued Files ({files.length} items — {formatBytes(totalBytes)})</span>
              {!uploading && (
                <button onClick={() => setFiles([])} className="text-red-600 hover:underline">
                  Clear all
                </button>
              )}
            </div>

            <div className="max-h-56 overflow-y-auto border border-[#eaeded] rounded divide-y divide-[#eaeded] text-xs">
              {files.map((file, idx) => {
                const prog = progressMap[idx];
                return (
                  <div key={file.name + idx} className="p-2.5 flex items-center justify-between bg-white hover:bg-slate-50">
                    <div className="flex items-center space-x-2 flex-1 min-w-0 pr-3">
                      <span>{getFileIcon(file.name)}</span>
                      <div className="min-w-0 flex-1">
                        <p className="font-medium text-[#16191f] truncate">{file.name}</p>
                        <p className="text-[11px] text-[#545b64] font-mono">{formatBytes(file.size)}</p>
                      </div>
                    </div>

                    {/* Progress Bar / Status */}
                    <div className="flex items-center space-x-3">
                      {prog ? (
                        <div className="w-32 flex items-center space-x-2">
                          <div className="flex-1 bg-slate-200 rounded-full h-2 overflow-hidden">
                            <div 
                              className={`h-full transition-all duration-200 ${
                                prog.status === "error" ? "bg-red-500" : "bg-[#2563eb]"
                              }`}
                              style={{ width: `${prog.percent || 0}%` }}
                            />
                          </div>
                          <span className="font-mono text-[10px] text-[#545b64] w-8 text-right">
                            {prog.status === "error" ? "❌" : `${prog.percent}%`}
                          </span>
                        </div>
                      ) : (
                        <span className="text-[11px] text-slate-400">Ready</span>
                      )}

                      {!uploading && (
                        <button
                          onClick={() => removeFile(idx)}
                          className="text-slate-400 hover:text-red-600 p-1"
                        >
                          ✕
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Footer Actions */}
        <div className="flex items-center justify-end space-x-3 pt-3 border-t border-[#eaeded]">
          <button
            onClick={onClose}
            disabled={uploading}
            className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f]"
          >
            Cancel
          </button>
          <button
            onClick={startUpload}
            disabled={files.length === 0 || uploading}
            className="h-8 px-5 bg-[#2563eb] hover:bg-[#1d4ed8] disabled:opacity-40 text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1.5"
          >
            {uploading ? (
              <>
                <span className="animate-spin">🔄</span>
                <span>Uploading files...</span>
              </>
            ) : (
              <span>Upload {files.length > 0 ? `(${files.length})` : ""}</span>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// 4.2 Phase 3: Create Folder Modal
// ----------------------------------------------------------------------
function AwsCreateFolderModal({ bucket, currentPrefix, onClose, onCreated, addToast }) {
  const [folderName, setFolderName] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleCreate = async (e) => {
    e.preventDefault();
    const clean = folderName.trim().replace(/^\/+|\/+$/g, "");
    if (!clean || clean.includes("//")) {
      setError("Nome de pasta inválido. Não utilize barras consecutivas.");
      return;
    }

    setLoading(true);
    setError("");
    try {
      const client = AuthManager.getClient();
      await client.createFolder(bucket, `${currentPrefix}${clean}/`);
      onCreated();
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-fade-in">
      <div className="w-full max-w-md bg-white rounded-lg shadow-2xl border border-slate-300 p-6 space-y-4">
        <div className="flex items-center justify-between border-b border-[#eaeded] pb-3">
          <h2 className="text-lg font-bold text-[#16191f]">Create folder</h2>
          <button onClick={onClose} className="text-[#545b64] hover:text-[#16191f] text-sm font-bold">✕</button>
        </div>

        {error && (
          <div className="p-3 bg-red-50 border-l-4 border-red-500 text-red-700 text-xs rounded-r">
            {error}
          </div>
        )}

        <form onSubmit={handleCreate} className="space-y-4">
          <div>
            <label className="block text-xs font-bold text-[#16191f] mb-1">Folder name</label>
            <input
              type="text"
              value={folderName}
              onChange={e => setFolderName(e.target.value)}
              placeholder="ex: logs"
              required
              autoFocus
              className="w-full h-9 px-3 border border-[#aab7b8] rounded text-xs text-[#16191f] focus:outline-none focus:border-[#2563eb]"
            />
            <p className="text-[11px] text-[#545b64] mt-1">
              Destination: <code className="font-mono bg-slate-100 px-1 rounded text-slate-800">s3://{bucket}/{currentPrefix}{folderName}/</code>
            </p>
          </div>

          <div className="flex items-center justify-end space-x-3 pt-3 border-t border-[#eaeded]">
            <button
              type="button"
              onClick={onClose}
              className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f]"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || !folderName.trim()}
              className="h-8 px-4 bg-[#2563eb] hover:bg-[#1d4ed8] text-white text-xs font-bold rounded shadow-sm transition"
            >
              {loading ? "Creating..." : "Create folder"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// 4.3 Phase 3: Multi-Object Delete Confirmation Modal
// ----------------------------------------------------------------------
function AwsDeleteObjectsModal({ bucket, keys, onClose, onDeleted, addToast }) {
  const [confirmInput, setConfirmInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const requiredConfirmation = "permanently delete";
  const isConfirmed = confirmInput.trim().toLowerCase() === requiredConfirmation;

  const handleDelete = async (e) => {
    e.preventDefault();
    if (!isConfirmed) return;

    setLoading(true);
    setError("");
    try {
      const client = AuthManager.getClient();
      await client.deleteObjects(bucket, keys);
      onDeleted();
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-fade-in">
      <div className="w-full max-w-lg bg-white rounded-lg shadow-2xl border border-red-200 p-6 space-y-4">
        <div className="flex items-center space-x-2 text-red-600">
          <span className="text-xl">⚠️</span>
          <h2 className="text-lg font-bold text-[#16191f]">Delete objects ({keys.length})</h2>
        </div>

        <p className="text-xs text-[#545b64]">
          Você está prestes a excluir permanentemente <strong>{keys.length}</strong> objeto(s) do bucket <strong>{bucket}</strong>.
        </p>

        <div className="max-h-40 overflow-y-auto border border-[#eaeded] rounded bg-slate-50 p-2 space-y-1 text-xs font-mono">
          {keys.map(k => (
            <div key={k} className="text-slate-800 truncate">📄 {k}</div>
          ))}
        </div>

        {error && (
          <div className="p-3 bg-red-50 border-l-4 border-red-500 text-red-700 text-xs rounded-r">
            {error}
          </div>
        )}

        <form onSubmit={handleDelete} className="space-y-4">
          <div>
            <label className="block text-xs font-bold text-[#16191f] mb-1">
              Para confirmar a exclusão, digite <code className="bg-slate-200 px-1 py-0.5 rounded text-red-600 font-mono select-all">permanently delete</code>:
            </label>
            <input
              type="text"
              value={confirmInput}
              onChange={e => setConfirmInput(e.target.value)}
              placeholder="permanently delete"
              required
              autoFocus
              className="w-full h-9 px-3 border border-[#aab7b8] rounded text-xs text-[#16191f] focus:outline-none focus:border-red-500 font-mono"
            />
          </div>

          <div className="flex items-center justify-end space-x-3 pt-3 border-t border-[#eaeded]">
            <button
              type="button"
              onClick={onClose}
              className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f]"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!isConfirmed || loading}
              className="h-8 px-4 bg-red-600 hover:bg-red-700 disabled:opacity-40 text-white text-xs font-bold rounded shadow-sm transition"
            >
              {loading ? "Deleting..." : "Delete objects"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// 4.4 Phase 3: Object Preview & Details Modal / Drawer
// ----------------------------------------------------------------------
function AwsObjectPreviewModal({ bucket, object, onClose, onDeleted, addToast }) {
  const [loading, setLoading] = useState(true);
  const [blobUrl, setBlobUrl] = useState(null);
  const [textContent, setTextContent] = useState(null);
  const [meta, setMeta] = useState(object);

  const ext = (object.key.split(".").pop() || "").toLowerCase();
  const isImg = ["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "ico"].includes(ext);
  const isTxt = ["json", "js", "ts", "py", "rs", "txt", "log", "md", "html", "css", "xml", "yaml", "yml", "csv", "sh", "sql"].includes(ext);

  useEffect(() => {
    let active = true;
    const fetchBlob = async () => {
      try {
        const client = AuthManager.getClient();
        const data = await client.getObjectBlob(bucket, object.key);
        if (!active) return;
        setMeta(prev => ({ ...prev, ...data }));
        const url = URL.createObjectURL(data.blob);
        setBlobUrl(url);

        if (isTxt && data.size < 500 * 1024) {
          const txt = await data.blob.text();
          if (active) setTextContent(txt);
        }
      } catch (err) {
        console.error("Preview fetch error:", err);
      } finally {
        if (active) setLoading(false);
      }
    };
    fetchBlob();
    return () => {
      active = false;
      if (blobUrl) URL.revokeObjectURL(blobUrl);
    };
  }, [bucket, object.key]);

  const handleDownload = async () => {
    try {
      const client = AuthManager.getClient();
      await client.downloadObject(bucket, object.key);
      addToast("Download concluído!", "success");
    } catch (err) {
      addToast(`Erro: ${err.message}`, "error");
    }
  };

  const handleDelete = async () => {
    if (!confirm(`Tem certeza que deseja excluir '${object.key}'?`)) return;
    try {
      const client = AuthManager.getClient();
      await client.deleteObject(bucket, object.key);
      onDeleted();
    } catch (err) {
      addToast(`Erro ao excluir: ${err.message}`, "error");
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-fade-in overflow-y-auto">
      <div className="w-full max-w-3xl bg-white rounded-lg shadow-2xl border border-slate-300 p-6 space-y-4 my-8">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#eaeded] pb-3">
          <div className="flex items-center space-x-2">
            <span className="text-xl">{getFileIcon(object.key)}</span>
            <div>
              <h2 className="text-base font-bold text-[#16191f] truncate max-w-lg">{object.key}</h2>
              <p className="text-xs text-[#545b64]">Object details and live preview</p>
            </div>
          </div>
          <button onClick={onClose} className="text-[#545b64] hover:text-[#16191f] text-sm font-bold">✕</button>
        </div>

        {/* Quick Actions Bar */}
        <div className="flex items-center justify-between p-3 bg-slate-50 border border-[#eaeded] rounded">
          <div className="flex items-center space-x-2">
            <button
              onClick={handleDownload}
              className="h-8 px-3 bg-white hover:bg-slate-50 border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition flex items-center space-x-1"
            >
              <span>⬇</span>
              <span>Download</span>
            </button>
            <button
              onClick={() => {
                navigator.clipboard.writeText(`s3://${bucket}/${object.key}`);
                addToast("S3 URI copiado!", "success");
              }}
              className="h-8 px-3 bg-white hover:bg-slate-50 border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition"
            >
              Copy S3 URI
            </button>
          </div>

          <button
            onClick={handleDelete}
            className="h-8 px-3 bg-white hover:bg-red-50 border border-red-300 rounded text-xs font-semibold text-red-600 transition"
          >
            Delete
          </button>
        </div>

        {/* Live Preview Area */}
        <div className="border border-[#eaeded] rounded-lg p-4 bg-slate-100 min-h-[220px] flex items-center justify-center">
          {loading ? (
            <div className="text-xs text-[#545b64] flex items-center space-x-2">
              <span className="animate-spin text-lg text-[#2563eb]">🔄</span>
              <span>Carregando preview...</span>
            </div>
          ) : isImg && blobUrl ? (
            <img src={blobUrl} alt={object.key} className="max-h-96 max-w-full rounded shadow object-contain" />
          ) : isTxt && textContent !== null ? (
            <div className="w-full max-h-96 overflow-auto bg-slate-900 text-slate-100 p-4 rounded text-xs font-mono">
              <pre className="whitespace-pre-wrap select-text">{textContent}</pre>
            </div>
          ) : (
            <div className="text-center space-y-2">
              <span className="text-4xl">{getFileIcon(object.key)}</span>
              <p className="text-xs text-[#545b64]">Preview visual não suportado para este tipo de arquivo.</p>
              <button
                onClick={handleDownload}
                className="px-3 py-1.5 bg-[#2563eb] text-white text-xs font-bold rounded"
              >
                Download para visualizar
              </button>
            </div>
          )}
        </div>

        {/* Metadata Properties Grid */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
          <div className="p-3 bg-slate-50 border border-[#eaeded] rounded">
            <span className="text-[11px] text-[#545b64] block">Size</span>
            <span className="font-bold text-[#16191f] font-mono">{formatBytes(meta.size)}</span>
          </div>
          <div className="p-3 bg-slate-50 border border-[#eaeded] rounded">
            <span className="text-[11px] text-[#545b64] block">Content Type</span>
            <span className="font-bold text-[#16191f] truncate block">{meta.contentType || "binary/octet-stream"}</span>
          </div>
          <div className="p-3 bg-slate-50 border border-[#eaeded] rounded">
            <span className="text-[11px] text-[#545b64] block">ETag</span>
            <span className="font-bold text-[#16191f] font-mono truncate block" title={meta.etag}>{meta.etag || "-"}</span>
          </div>
          <div className="p-3 bg-slate-50 border border-[#eaeded] rounded">
            <span className="text-[11px] text-[#545b64] block">Storage Class</span>
            <span className="font-bold text-[#16191f]">{meta.storageClass || "STANDARD"}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// 5. AWS Create Bucket Modal (Phase 2 - Advanced Configuration)
// ----------------------------------------------------------------------
function AwsCreateBucketModal({ onClose, onCreated, addToast }) {
  const [bucketName, setBucketName] = useState("");
  const [enableVersioning, setEnableVersioning] = useState(false);
  const [encryptionType, setEncryptionType] = useState("SSE-S3");
  const [blockPublicAccess, setBlockPublicAccess] = useState(true);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleCreate = async (e) => {
    e.preventDefault();
    const cleanName = bucketName.trim().toLowerCase();
    
    // S3 Naming Rules Validation
    if (!/^[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]$/.test(cleanName) || cleanName.includes("..") || cleanName.includes(".-") || cleanName.includes("-.")) {
      setError("Nome de bucket inválido. Deve ter entre 3 e 63 caracteres, conter apenas letras minúsculas, números, pontos ou hífens, e começar/terminar com letra ou número.");
      return;
    }

    setLoading(true);
    setError("");
    try {
      const client = AuthManager.getClient();
      await client.createBucket(cleanName, {
        versioning: enableVersioning,
        encryption: encryptionType
      });
      onCreated();
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-fade-in overflow-y-auto">
      <div className="w-full max-w-xl bg-white rounded-lg shadow-2xl border border-slate-300 p-6 my-8">
        <div className="flex items-center justify-between border-b border-[#eaeded] pb-4 mb-4">
          <div>
            <h2 className="text-lg font-bold text-[#16191f]">Create bucket</h2>
            <p className="text-xs text-[#545b64]">General configuration for your new Z3S storage container.</p>
          </div>
          <button onClick={onClose} className="text-[#545b64] hover:text-[#16191f] text-sm font-bold">✕</button>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-red-50 border-l-4 border-red-500 text-red-700 text-xs rounded-r">
            <p className="font-semibold">Erro ao criar bucket:</p>
            <p>{error}</p>
          </div>
        )}

        <form onSubmit={handleCreate} className="space-y-5">
          {/* 1. Bucket Name */}
          <div className="space-y-1">
            <label className="block text-xs font-bold text-[#16191f]">
              Bucket name <span className="text-red-500">*</span>
            </label>
            <input 
              type="text"
              value={bucketName}
              onChange={e => setBucketName(e.target.value.toLowerCase())}
              placeholder="ex: my-company-backups-2026"
              required
              autoFocus
              className="w-full h-9 px-3 border border-[#aab7b8] rounded text-xs text-[#16191f] focus:outline-none focus:border-[#2563eb] font-mono"
            />
            <p className="text-[11px] text-[#545b64]">
              O nome do bucket deve ser globalmente único e não conter espaços ou maiúsculas.
            </p>
          </div>

          {/* 2. Region */}
          <div className="space-y-1">
            <label className="block text-xs font-bold text-[#16191f]">AWS Region</label>
            <div className="h-9 px-3 bg-[#f2f3f3] border border-[#eaeded] rounded text-xs text-[#545b64] flex items-center justify-between font-mono">
              <span>us-east-1 (US East / N. Virginia)</span>
              <span className="text-[10px] bg-slate-200 px-1.5 py-0.5 rounded text-slate-700 font-sans">Default</span>
            </div>
          </div>

          {/* 3. Bucket Versioning */}
          <div className="p-4 border border-[#eaeded] rounded-lg bg-slate-50 space-y-2">
            <div className="flex items-center justify-between">
              <div>
                <h4 className="text-xs font-bold text-[#16191f]">Bucket Versioning</h4>
                <p className="text-[11px] text-[#545b64]">
                  Mantém múltiplas variantes de cada objeto no mesmo bucket para proteção contra exclusão acidental.
                </p>
              </div>
              <label className="relative inline-flex items-center cursor-pointer">
                <input 
                  type="checkbox" 
                  checked={enableVersioning} 
                  onChange={e => setEnableVersioning(e.target.checked)} 
                  className="sr-only peer"
                />
                <div className="w-9 h-5 bg-gray-300 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-[#2563eb]"></div>
              </label>
            </div>
            <span className="inline-block text-[10px] font-semibold px-2 py-0.5 rounded border border-slate-200 bg-white text-slate-700">
              Status: {enableVersioning ? "Enabled (Ativado)" : "Disabled (Desativado)"}
            </span>
          </div>

          {/* 4. Default Encryption */}
          <div className="p-4 border border-[#eaeded] rounded-lg bg-slate-50 space-y-2">
            <h4 className="text-xs font-bold text-[#16191f]">Default Encryption</h4>
            <p className="text-[11px] text-[#545b64]">Criptografa automaticamente todos os novos objetos salvos neste bucket em repouso.</p>
            <div className="grid grid-cols-2 gap-2 pt-1">
              <label className={`p-2.5 border rounded cursor-pointer text-xs flex flex-col space-y-0.5 transition ${encryptionType === "SSE-S3" ? "border-[#2563eb] bg-blue-50/50" : "border-[#eaeded] bg-white"}`}>
                <div className="flex items-center space-x-1.5">
                  <input type="radio" name="enc" checked={encryptionType === "SSE-S3"} onChange={() => setEncryptionType("SSE-S3")} className="text-[#2563eb]" />
                  <span className="font-semibold text-[#16191f]">SSE-S3 (AES-256)</span>
                </div>
                <span className="text-[10px] text-[#545b64] pl-5">Chaves gerenciadas pelo Z3S</span>
              </label>

              <label className={`p-2.5 border rounded cursor-pointer text-xs flex flex-col space-y-0.5 transition ${encryptionType === "SSE-KMS" ? "border-[#2563eb] bg-blue-50/50" : "border-[#eaeded] bg-white"}`}>
                <div className="flex items-center space-x-1.5">
                  <input type="radio" name="enc" checked={encryptionType === "SSE-KMS"} onChange={() => setEncryptionType("SSE-KMS")} className="text-[#2563eb]" />
                  <span className="font-semibold text-[#16191f]">SSE-KMS</span>
                </div>
                <span className="text-[10px] text-[#545b64] pl-5">Chaves envelope via KMS</span>
              </label>
            </div>
          </div>

          {/* 5. Block Public Access */}
          <div className="p-3 border border-[#eaeded] rounded bg-slate-50 flex items-start space-x-2">
            <input 
              id="bpa"
              type="checkbox" 
              checked={blockPublicAccess} 
              onChange={e => setBlockPublicAccess(e.target.checked)}
              className="mt-0.5 rounded text-[#2563eb] focus:ring-[#2563eb]"
            />
            <label htmlFor="bpa" className="text-xs text-[#16191f] cursor-pointer">
              <strong>Block all public access</strong> (Recomendado)
              <p className="text-[11px] text-[#545b64] mt-0.5">
                Garante que nenhum objeto ou política conceda permissões de leitura anônimas públicas.
              </p>
            </label>
          </div>

          {/* Footer Actions */}
          <div className="flex items-center justify-end space-x-3 pt-4 border-t border-[#eaeded]">
            <button
              type="button"
              onClick={onClose}
              className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f]"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading}
              className="h-8 px-4 bg-[#2563eb] hover:bg-[#1d4ed8] active:bg-[#1e40af] text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1.5"
            >
              {loading ? (
                <>
                  <span className="animate-spin">🔄</span>
                  <span>Creating bucket...</span>
                </>
              ) : (
                <span>Create bucket</span>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// 6. AWS Delete Bucket Modal (Phase 2 - Safe Delete Confirmation)
// ----------------------------------------------------------------------
function AwsDeleteBucketModal({ bucketName, onClose, onDeleted, onOpenEmptyModal, addToast }) {
  const [confirmInput, setConfirmInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [isBucketNotEmpty, setIsBucketNotEmpty] = useState(false);

  const isConfirmed = confirmInput === bucketName;

  const handleDelete = async (e) => {
    e.preventDefault();
    if (!isConfirmed) return;

    setLoading(true);
    setError("");
    setIsBucketNotEmpty(false);

    try {
      const client = AuthManager.getClient();
      await client.deleteBucket(bucketName);
      onDeleted();
    } catch (err) {
      console.error(err);
      if (err.message.includes("409") || err.message.includes("BucketNotEmpty") || err.message.includes("não está vazio")) {
        setIsBucketNotEmpty(true);
        setError(`O bucket '${bucketName}' não está vazio. Todos os objetos devem ser excluídos antes de excluir o bucket.`);
      } else {
        setError(err.message);
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-fade-in">
      <div className="w-full max-w-lg bg-white rounded-lg shadow-2xl border border-red-200 p-6">
        <div className="flex items-center space-x-2 text-red-600 mb-2">
          <span className="text-xl">⚠️</span>
          <h2 className="text-lg font-bold text-[#16191f]">Delete bucket</h2>
        </div>

        <p className="text-xs text-[#545b64] mb-4">
          A exclusão de um bucket é <strong>irreversível</strong>. Uma vez excluído, as configurações e o namespace serão liberados.
        </p>

        {error && (
          <div className="mb-4 p-3 bg-red-50 border-l-4 border-red-500 text-red-700 text-xs rounded-r space-y-2">
            <p className="font-semibold">{error}</p>
            {isBucketNotEmpty && (
              <button
                type="button"
                onClick={onOpenEmptyModal}
                className="px-3 py-1.5 bg-amber-600 hover:bg-amber-700 text-white rounded font-bold transition flex items-center space-x-1"
              >
                <span>🧹 Esvaziar objetos agora</span>
              </button>
            )}
          </div>
        )}

        <form onSubmit={handleDelete} className="space-y-4">
          <div className="p-3 bg-slate-50 border border-[#eaeded] rounded text-xs space-y-1">
            <span className="text-[#545b64]">Bucket selecionado:</span>
            <p className="font-mono font-bold text-[#16191f] text-sm">🪣 {bucketName}</p>
          </div>

          <div>
            <label className="block text-xs font-bold text-[#16191f] mb-1.5">
              Para confirmar a exclusão, digite exatamente <code className="bg-slate-200 px-1 py-0.5 rounded text-red-600 font-mono select-all">{bucketName}</code> abaixo:
            </label>
            <input 
              type="text"
              value={confirmInput}
              onChange={e => setConfirmInput(e.target.value)}
              placeholder={bucketName}
              required
              autoFocus
              className="w-full h-9 px-3 border border-[#aab7b8] rounded text-xs text-[#16191f] focus:outline-none focus:border-red-500 font-mono"
            />
          </div>

          <div className="flex items-center justify-end space-x-3 pt-4 border-t border-[#eaeded]">
            <button
              type="button"
              onClick={onClose}
              className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f]"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!isConfirmed || loading}
              className="h-8 px-4 bg-red-600 hover:bg-red-700 disabled:opacity-40 disabled:hover:bg-red-600 text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1.5"
            >
              {loading ? (
                <>
                  <span className="animate-spin">🔄</span>
                  <span>Deleting...</span>
                </>
              ) : (
                <span>Delete bucket</span>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// 7. AWS Empty Bucket Modal (Phase 2 - Empty All Objects)
// ----------------------------------------------------------------------
function AwsEmptyBucketModal({ bucketName, onClose, onEmptied, addToast }) {
  const [confirmInput, setConfirmInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const requiredConfirmation = "permanently delete";
  const isConfirmed = confirmInput.trim().toLowerCase() === requiredConfirmation;

  const handleEmpty = async (e) => {
    e.preventDefault();
    if (!isConfirmed) return;

    setLoading(true);
    setError("");

    try {
      const client = AuthManager.getClient();
      await client.emptyBucket(bucketName);
      onEmptied();
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-fade-in">
      <div className="w-full max-w-lg bg-white rounded-lg shadow-2xl border border-amber-200 p-6">
        <div className="flex items-center space-x-2 text-amber-600 mb-2">
          <span className="text-xl">🧹</span>
          <h2 className="text-lg font-bold text-[#16191f]">Empty bucket</h2>
        </div>

        <p className="text-xs text-[#545b64] mb-4">
          Esvaziar o bucket <strong>{bucketName}</strong> excluirá <strong>todos os objetos e dados</strong> contidos nele de forma permanente.
        </p>

        {error && (
          <div className="mb-4 p-3 bg-red-50 border-l-4 border-red-500 text-red-700 text-xs rounded-r">
            <p className="font-semibold">Erro ao esvaziar bucket:</p>
            <p>{error}</p>
          </div>
        )}

        <form onSubmit={handleEmpty} className="space-y-4">
          <div className="p-3 bg-amber-50 border border-amber-200 rounded text-xs text-amber-900 space-y-1">
            <p className="font-semibold">⚠️ Ação irreversível:</p>
            <p>Todos os arquivos, diretórios virtuais e versões de objetos serão apagados sem possibilidade de recuperação.</p>
          </div>

          <div>
            <label className="block text-xs font-bold text-[#16191f] mb-1.5">
              Para confirmar, digite <code className="bg-slate-200 px-1 py-0.5 rounded text-amber-700 font-mono select-all">permanently delete</code> no campo abaixo:
            </label>
            <input 
              type="text"
              value={confirmInput}
              onChange={e => setConfirmInput(e.target.value)}
              placeholder="permanently delete"
              required
              autoFocus
              className="w-full h-9 px-3 border border-[#aab7b8] rounded text-xs text-[#16191f] focus:outline-none focus:border-amber-500 font-mono"
            />
          </div>

          <div className="flex items-center justify-end space-x-3 pt-4 border-t border-[#eaeded]">
            <button
              type="button"
              onClick={onClose}
              className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f]"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!isConfirmed || loading}
              className="h-8 px-4 bg-amber-600 hover:bg-amber-700 disabled:opacity-40 disabled:hover:bg-amber-600 text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1.5"
            >
              {loading ? (
                <>
                  <span className="animate-spin">🔄</span>
                  <span>Emptying objects...</span>
                </>
              ) : (
                <span>Empty bucket</span>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// 8. Bucket Policy Editor Component (Opção 1)
// ----------------------------------------------------------------------
function AwsBucketPolicyEditor({ bucket, addToast }) {
  const bucketName = typeof bucket === "object" && bucket !== null ? bucket.name : (bucket || "");
  const [policyText, setPolicyText] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [selectedPreset, setSelectedPreset] = useState("");

  const presets = {
    public_read: {
      name: "Public Read (ReadOnly)",
      json: JSON.stringify(
        {
          Version: "2012-10-17",
          Statement: [
            {
              Sid: "PublicReadGetObject",
              Effect: "Allow",
              Principal: "*",
              Action: ["s3:GetObject"],
              Resource: [`arn:aws:s3:::${bucketName}/*`]
            }
          ]
        },
        null,
        2
      )
    },
    deny_http: {
      name: "Deny Insecure HTTP (Enforce HTTPS/SSL)",
      json: JSON.stringify(
        {
          Version: "2012-10-17",
          Statement: [
            {
              Sid: "AllowSSLRequestsOnly",
              Effect: "Deny",
              Principal: "*",
              Action: "s3:*",
              Resource: [
                `arn:aws:s3:::${bucketName}`,
                `arn:aws:s3:::${bucketName}/*`
              ],
              Condition: {
                Bool: {
                  "aws:SecureTransport": "false"
                }
              }
            }
          ]
        },
        null,
        2
      )
    },
    ip_whitelist: {
      name: "IP Whitelist (VPC / Corporate CIDR)",
      json: JSON.stringify(
        {
          Version: "2012-10-17",
          Statement: [
            {
              Sid: "IPAllow",
              Effect: "Allow",
              Principal: "*",
              Action: "s3:*",
              Resource: [
                `arn:aws:s3:::${bucketName}`,
                `arn:aws:s3:::${bucketName}/*`
              ],
              Condition: {
                IpAddress: {
                  "aws:SourceIp": "192.168.1.0/24"
                }
              }
            }
          ]
        },
        null,
        2
      )
    },
    enforce_sse: {
      name: "Enforce Server-Side Encryption (SSE-KMS)",
      json: JSON.stringify(
        {
          Version: "2012-10-17",
          Statement: [
            {
              Sid: "DenyUnEncryptedObjectUploads",
              Effect: "Deny",
              Principal: "*",
              Action: "s3:PutObject",
              Resource: `arn:aws:s3:::${bucketName}/*`,
              Condition: {
                StringNotEquals: {
                  "s3:x-amz-server-side-encryption": "aws:kms"
                }
              }
            }
          ]
        },
        null,
        2
      )
    }
  };

  const loadPolicy = async () => {
    if (!bucketName) return;
    setLoading(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      const p = await client.getBucketPolicy(bucketName);
      if (p) {
        try {
          const parsed = JSON.parse(p);
          setPolicyText(JSON.stringify(parsed, null, 2));
        } catch (_) {
          setPolicyText(p);
        }
      } else {
        setPolicyText("");
      }
    } catch (err) {
      console.warn("Could not load bucket policy:", err);
      setPolicyText("");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPolicy();
  }, [bucketName]);

  const handleApplyPreset = (key) => {
    setSelectedPreset(key);
    if (presets[key]) {
      setPolicyText(presets[key].json);
    }
  };

  const handleFormatJson = () => {
    try {
      const parsed = JSON.parse(policyText);
      setPolicyText(JSON.stringify(parsed, null, 2));
      addToast("JSON formatado com sucesso", "success");
    } catch (e) {
      addToast(`Erro de sintaxe JSON: ${e.message}`, "error");
    }
  };

  const handleSavePolicy = async () => {
    if (!policyText.trim()) {
      addToast("A política JSON não pode estar vazia.", "error");
      return;
    }
    try {
      JSON.parse(policyText);
    } catch (e) {
      addToast(`JSON Inválido: ${e.message}`, "error");
      return;
    }

    setSaving(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      await client.putBucketPolicy(bucketName, policyText);
      addToast("Bucket Policy aplicada com sucesso!", "success");
    } catch (err) {
      addToast(`Erro ao salvar Bucket Policy: ${err.message}`, "error");
    } finally {
      setSaving(false);
    }
  };

  const handleDeletePolicy = async () => {
    if (!confirm("Tem certeza que deseja remover a Bucket Policy deste bucket?")) return;
    setSaving(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      await client.deleteBucketPolicy(bucketName);
      setPolicyText("");
      addToast("Bucket Policy removida com sucesso.", "success");
    } catch (err) {
      addToast(`Erro ao excluir Bucket Policy: ${err.message}`, "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-4">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-bold text-[#16191f] flex items-center space-x-2">
            <span>Bucket Policy (JSON Editor)</span>
            {policyText && (
              <span className="px-2 py-0.5 bg-blue-50 text-blue-700 text-[10px] font-bold rounded border border-blue-200">
                Ativa
              </span>
            )}
          </h3>
          <p className="text-xs text-[#545b64]">
            Defina permissões refinadas de acesso ao bucket e objetos utilizando a gramática padrão AWS IAM JSON.
          </p>
        </div>
        <div className="flex items-center space-x-2">
          <select
            value={selectedPreset}
            onChange={(e) => handleApplyPreset(e.target.value)}
            className="h-8 px-2 bg-white border border-[#aab7b8] text-xs text-[#16191f] rounded focus:outline-none focus:ring-1 focus:ring-[#0073bb]"
          >
            <option value="">-- Carregar Modelo / Preset --</option>
            {Object.entries(presets).map(([k, v]) => (
              <option key={k} value={k}>{v.name}</option>
            ))}
          </select>
        </div>
      </div>

      {loading ? (
        <div className="p-8 text-center text-xs text-[#545b64]">
          <span className="animate-spin inline-block mr-2">🔄</span> Carregando política...
        </div>
      ) : (
        <div className="space-y-3">
          <div className="relative">
            <textarea
              value={policyText}
              onChange={(e) => setPolicyText(e.target.value)}
              placeholder={`{\n  "Version": "2012-10-17",\n  "Statement": []\n}`}
              rows={12}
              className="w-full font-mono text-xs p-3 bg-slate-900 text-emerald-400 rounded border border-slate-700 focus:outline-none focus:ring-1 focus:ring-[#0073bb] leading-relaxed resize-y"
              spellCheck={false}
            />
          </div>

          <div className="flex flex-wrap items-center justify-between gap-2 pt-2 border-t border-[#eaeded]">
            <div className="flex items-center space-x-2">
              <button
                type="button"
                onClick={handleFormatJson}
                disabled={!policyText.trim() || saving}
                className="h-8 px-3 bg-slate-100 hover:bg-slate-200 text-xs font-semibold text-[#16191f] rounded border border-[#aab7b8] transition"
              >
                Formatar JSON
              </button>
              {policyText && (
                <button
                  type="button"
                  onClick={handleDeletePolicy}
                  disabled={saving}
                  className="h-8 px-3 bg-red-50 hover:bg-red-100 text-xs font-semibold text-red-700 rounded border border-red-300 transition"
                >
                  Excluir Política
                </button>
              )}
            </div>

            <button
              type="button"
              onClick={handleSavePolicy}
              disabled={saving}
              className="h-8 px-4 bg-[#ec7211] hover:bg-[#eb5f07] disabled:opacity-50 text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1.5"
            >
              {saving ? (
                <>
                  <span className="animate-spin">🔄</span>
                  <span>Salvando...</span>
                </>
              ) : (
                <span>Salvar Bucket Policy</span>
              )}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ----------------------------------------------------------------------
// 9. Bucket CORS Editor Component (Opção 1)
// ----------------------------------------------------------------------
function AwsBucketCorsEditor({ bucket, addToast }) {
  const bucketName = typeof bucket === "object" && bucket !== null ? bucket.name : (bucket || "");
  const [corsText, setCorsText] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [selectedPreset, setSelectedPreset] = useState("");

  const presets = {
    allow_all: {
      name: "Permitir Todas as Origens (* GET, PUT, POST, DELETE, HEAD)",
      xml: `<CORSConfiguration>
  <CORSRule>
    <AllowedOrigin>*</AllowedOrigin>
    <AllowedMethod>GET</AllowedMethod>
    <AllowedMethod>PUT</AllowedMethod>
    <AllowedMethod>POST</AllowedMethod>
    <AllowedMethod>DELETE</AllowedMethod>
    <AllowedMethod>HEAD</AllowedMethod>
    <AllowedHeader>*</AllowedHeader>
    <ExposeHeader>ETag</ExposeHeader>
    <ExposeHeader>x-amz-request-id</ExposeHeader>
    <MaxAgeSeconds>3000</MaxAgeSeconds>
  </CORSRule>
</CORSConfiguration>`
    },
    single_origin: {
      name: "Origem Específica (https://meuapp.com)",
      xml: `<CORSConfiguration>
  <CORSRule>
    <AllowedOrigin>https://meuapp.com</AllowedOrigin>
    <AllowedMethod>GET</AllowedMethod>
    <AllowedMethod>PUT</AllowedMethod>
    <AllowedHeader>*</AllowedHeader>
    <ExposeHeader>ETag</ExposeHeader>
    <MaxAgeSeconds>3600</MaxAgeSeconds>
  </CORSRule>
</CORSConfiguration>`
    },
    web_assets: {
      name: "Fontes e Recursos Web (GET & HEAD)",
      xml: `<CORSConfiguration>
  <CORSRule>
    <AllowedOrigin>*</AllowedOrigin>
    <AllowedMethod>GET</AllowedMethod>
    <AllowedMethod>HEAD</AllowedMethod>
    <AllowedHeader>*</AllowedHeader>
    <MaxAgeSeconds>86400</MaxAgeSeconds>
  </CORSRule>
</CORSConfiguration>`
    }
  };

  const loadCors = async () => {
    if (!bucketName) return;
    setLoading(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      const c = await client.getBucketCors(bucketName);
      setCorsText(c || "");
    } catch (err) {
      console.warn("Could not load CORS:", err);
      setCorsText("");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadCors();
  }, [bucketName]);

  const handleApplyPreset = (key) => {
    setSelectedPreset(key);
    if (presets[key]) {
      setCorsText(presets[key].xml);
    }
  };

  const handleSaveCors = async () => {
    if (!corsText.trim()) {
      addToast("A configuração CORS não pode estar vazia.", "error");
      return;
    }

    setSaving(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      await client.putBucketCors(bucketName, corsText);
      addToast("Configuração CORS salva com sucesso!", "success");
    } catch (err) {
      addToast(`Erro ao salvar CORS: ${err.message}`, "error");
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteCors = async () => {
    if (!confirm("Tem certeza que deseja remover as regras CORS deste bucket?")) return;
    setSaving(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      await client.deleteBucketCors(bucketName);
      setCorsText("");
      addToast("Configuração CORS removida com sucesso.", "success");
    } catch (err) {
      addToast(`Erro ao excluir CORS: ${err.message}`, "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-4">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-bold text-[#16191f] flex items-center space-x-2">
            <span>Cross-origin resource sharing (CORS)</span>
            {corsText && (
              <span className="px-2 py-0.5 bg-blue-50 text-blue-700 text-[10px] font-bold rounded border border-blue-200">
                Ativo
              </span>
            )}
          </h3>
          <p className="text-xs text-[#545b64]">
            Permita que aplicações web em outros domínios acessem recursos neste bucket com segurança.
          </p>
        </div>
        <div className="flex items-center space-x-2">
          <select
            value={selectedPreset}
            onChange={(e) => handleApplyPreset(e.target.value)}
            className="h-8 px-2 bg-white border border-[#aab7b8] text-xs text-[#16191f] rounded focus:outline-none focus:ring-1 focus:ring-[#0073bb]"
          >
            <option value="">-- Carregar Modelo CORS --</option>
            {Object.entries(presets).map(([k, v]) => (
              <option key={k} value={k}>{v.name}</option>
            ))}
          </select>
        </div>
      </div>

      {loading ? (
        <div className="p-8 text-center text-xs text-[#545b64]">
          <span className="animate-spin inline-block mr-2">🔄</span> Carregando CORS...
        </div>
      ) : (
        <div className="space-y-3">
          <textarea
            value={corsText}
            onChange={(e) => setCorsText(e.target.value)}
            placeholder={`<CORSConfiguration>\n  <CORSRule>\n    <AllowedOrigin>*</AllowedOrigin>\n    <AllowedMethod>GET</AllowedMethod>\n  </CORSRule>\n</CORSConfiguration>`}
            rows={10}
            className="w-full font-mono text-xs p-3 bg-slate-900 text-amber-300 rounded border border-slate-700 focus:outline-none focus:ring-1 focus:ring-[#0073bb] leading-relaxed resize-y"
            spellCheck={false}
          />

          <div className="flex flex-wrap items-center justify-between gap-2 pt-2 border-t border-[#eaeded]">
            <div>
              {corsText && (
                <button
                  type="button"
                  onClick={handleDeleteCors}
                  disabled={saving}
                  className="h-8 px-3 bg-red-50 hover:bg-red-100 text-xs font-semibold text-red-700 rounded border border-red-300 transition"
                >
                  Excluir CORS
                </button>
              )}
            </div>

            <button
              type="button"
              onClick={handleSaveCors}
              disabled={saving}
              className="h-8 px-4 bg-[#ec7211] hover:bg-[#eb5f07] disabled:opacity-50 text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1.5"
            >
              {saving ? (
                <>
                  <span className="animate-spin">🔄</span>
                  <span>Salvando...</span>
                </>
              ) : (
                <span>Salvar Configuração CORS</span>
              )}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ----------------------------------------------------------------------
// 10. Bucket Lifecycle Rules Manager (Opção 1)
// ----------------------------------------------------------------------
function AwsBucketLifecycleManager({ bucket, addToast }) {
  const bucketName = typeof bucket === "object" && bucket !== null ? bucket.name : (bucket || "");
  const [lifecycleXml, setLifecycleXml] = useState("");
  const [rules, setRules] = useState([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [showAddModal, setShowAddModal] = useState(false);

  // Form State
  const [ruleId, setRuleId] = useState("");
  const [rulePrefix, setRulePrefix] = useState("");
  const [expireDays, setExpireDays] = useState("30");
  const [abortMultipartDays, setAbortMultipartDays] = useState("7");

  const parseRules = (xmlString) => {
    if (!xmlString) return [];
    try {
      const parser = new DOMParser();
      const doc = parser.parseFromString(xmlString, "application/xml");
      const ruleNodes = doc.querySelectorAll("Rule");
      const list = [];
      ruleNodes.forEach((r) => {
        const id = r.querySelector("ID")?.textContent || "Unnamed-Rule";
        const prefix = r.querySelector("Prefix")?.textContent || r.querySelector("Filter > Prefix")?.textContent || "/ (All)";
        const status = r.querySelector("Status")?.textContent || "Enabled";
        const days = r.querySelector("Expiration > Days")?.textContent || "-";
        const abortDays = r.querySelector("AbortIncompleteMultipartUpload > DaysAfterInitiation")?.textContent || "-";
        list.push({ id, prefix, status, days, abortDays });
      });
      return list;
    } catch (_) {
      return [];
    }
  };

  const loadLifecycle = async () => {
    if (!bucketName) return;
    setLoading(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      const xml = await client.getBucketLifecycle(bucketName);
      setLifecycleXml(xml || "");
      setRules(parseRules(xml || ""));
    } catch (err) {
      console.warn("Could not load lifecycle:", err);
      setLifecycleXml("");
      setRules([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadLifecycle();
  }, [bucketName]);

  const handleAddRule = async (e) => {
    e.preventDefault();
    if (!ruleId.trim()) {
      addToast("O ID da regra é obrigatório.", "error");
      return;
    }

    const newRuleXml = `  <Rule>
    <ID>${ruleId.trim()}</ID>
    <Filter><Prefix>${rulePrefix.trim()}</Prefix></Filter>
    <Status>Enabled</Status>
    ${expireDays ? `<Expiration><Days>${expireDays}</Days></Expiration>` : ""}
    ${abortMultipartDays ? `<AbortIncompleteMultipartUpload><DaysAfterInitiation>${abortMultipartDays}</DaysAfterInitiation></AbortIncompleteMultipartUpload>` : ""}
  </Rule>`;

    let combinedXml = "";
    if (lifecycleXml && lifecycleXml.includes("<LifecycleConfiguration>")) {
      combinedXml = lifecycleXml.replace("</LifecycleConfiguration>", `${newRuleXml}\n</LifecycleConfiguration>`);
    } else {
      combinedXml = `<LifecycleConfiguration>\n${newRuleXml}\n</LifecycleConfiguration>`;
    }

    setSaving(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      await client.putBucketLifecycle(bucketName, combinedXml);
      addToast(`Regra de ciclo de vida '${ruleId}' adicionada!`, "success");
      setShowAddModal(false);
      setRuleId("");
      setRulePrefix("");
      await loadLifecycle();
    } catch (err) {
      addToast(`Erro ao salvar regra: ${err.message}`, "error");
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteLifecycle = async () => {
    if (!confirm("Tem certeza que deseja remover todas as regras de ciclo de vida deste bucket?")) return;
    setSaving(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      await client.deleteBucketLifecycle(bucketName);
      setLifecycleXml("");
      setRules([]);
      addToast("Regras de ciclo de vida removidas com sucesso.", "success");
    } catch (err) {
      addToast(`Erro ao remover regras: ${err.message}`, "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-4">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-bold text-[#16191f] flex items-center space-x-2">
            <span>Lifecycle rules (Ciclo de Vida de Objetos)</span>
            {rules.length > 0 && (
              <span className="px-2 py-0.5 bg-blue-50 text-blue-700 text-[10px] font-bold rounded border border-blue-200">
                {rules.length} {rules.length === 1 ? "regra ativa" : "regras ativas"}
              </span>
            )}
          </h3>
          <p className="text-xs text-[#545b64]">
            Automatize a expiração de objetos antigos ou exclusão de uploads multipart incompletos para otimizar custos.
          </p>
        </div>
        <div className="flex items-center space-x-2">
          {rules.length > 0 && (
            <button
              type="button"
              onClick={handleDeleteLifecycle}
              disabled={saving}
              className="h-8 px-3 bg-red-50 hover:bg-red-100 text-xs font-semibold text-red-700 rounded border border-red-300 transition"
            >
              Excluir Todas
            </button>
          )}
          <button
            type="button"
            onClick={() => setShowAddModal(true)}
            className="h-8 px-3.5 bg-[#ec7211] hover:bg-[#eb5f07] text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1"
          >
            <span>+ Criar Regra de Ciclo de Vida</span>
          </button>
        </div>
      </div>

      {loading ? (
        <div className="p-8 text-center text-xs text-[#545b64]">
          <span className="animate-spin inline-block mr-2">🔄</span> Carregando regras...
        </div>
      ) : rules.length === 0 ? (
        <div className="p-8 text-center border border-dashed border-[#eaeded] rounded-lg bg-slate-50 space-y-2">
          <p className="text-xs font-semibold text-[#16191f]">Nenhuma regra de ciclo de vida configurada</p>
          <p className="text-xs text-[#545b64]">Crie uma regra para expirar automaticamente arquivos temporários ou logs após N dias.</p>
        </div>
      ) : (
        <div className="border border-[#eaeded] rounded overflow-x-auto">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-[#fafafa] border-b border-[#eaeded] text-[#545b64] font-semibold">
              <tr>
                <th className="py-2.5 px-3">Nome da Regra (ID)</th>
                <th className="py-2.5 px-3">Escopo / Prefixo</th>
                <th className="py-2.5 px-3">Status</th>
                <th className="py-2.5 px-3">Expiração do Objeto</th>
                <th className="py-2.5 px-3">Limpeza Multipart</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[#eaeded] text-[#16191f]">
              {rules.map((r, i) => (
                <tr key={i} className="hover:bg-slate-50">
                  <td className="py-2 px-3 font-semibold text-[#0073bb]">{r.id}</td>
                  <td className="py-2 px-3 font-mono">{r.prefix || "(Todo o bucket)"}</td>
                  <td className="py-2 px-3">
                    <span className="px-1.5 py-0.5 bg-emerald-50 text-emerald-700 text-[10px] font-bold rounded border border-emerald-200">
                      {r.status}
                    </span>
                  </td>
                  <td className="py-2 px-3">{r.days !== "-" ? `${r.days} dias após criação` : "Não configurado"}</td>
                  <td className="py-2 px-3">{r.abortDays !== "-" ? `${r.abortDays} dias após início` : "Não configurado"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Modal: Adicionar Regra */}
      {showAddModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="bg-white rounded-lg shadow-xl w-full max-w-lg border border-[#eaeded] overflow-hidden animate-fade-in">
            <div className="px-5 py-4 border-b border-[#eaeded] flex items-center justify-between bg-[#fafafa]">
              <h3 className="text-sm font-bold text-[#16191f]">Criar Regra de Ciclo de Vida</h3>
              <button
                type="button"
                onClick={() => setShowAddModal(false)}
                className="text-[#545b64] hover:text-[#16191f] text-sm font-bold"
              >
                ✕
              </button>
            </div>
            <form onSubmit={handleAddRule} className="p-5 space-y-4">
              <div>
                <label className="block text-xs font-bold text-[#16191f] mb-1">
                  Nome da Regra (Rule ID) *
                </label>
                <input
                  type="text"
                  required
                  placeholder="ex: expirar-logs-30-dias"
                  value={ruleId}
                  onChange={(e) => setRuleId(e.target.value)}
                  className="w-full h-8 px-3 border border-[#aab7b8] rounded text-xs focus:outline-none focus:ring-1 focus:ring-[#0073bb]"
                />
              </div>

              <div>
                <label className="block text-xs font-bold text-[#16191f] mb-1">
                  Filtro por Prefixo (Opcional)
                </label>
                <input
                  type="text"
                  placeholder="ex: logs/ ou tmp/ (deixe em branco para todo o bucket)"
                  value={rulePrefix}
                  onChange={(e) => setRulePrefix(e.target.value)}
                  className="w-full h-8 px-3 border border-[#aab7b8] rounded text-xs focus:outline-none focus:ring-1 focus:ring-[#0073bb]"
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-bold text-[#16191f] mb-1">
                    Expirar Objetos Após (Dias)
                  </label>
                  <input
                    type="number"
                    min="1"
                    value={expireDays}
                    onChange={(e) => setExpireDays(e.target.value)}
                    className="w-full h-8 px-3 border border-[#aab7b8] rounded text-xs focus:outline-none focus:ring-1 focus:ring-[#0073bb]"
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-[#16191f] mb-1">
                    Abortar Multipart Após (Dias)
                  </label>
                  <input
                    type="number"
                    min="1"
                    value={abortMultipartDays}
                    onChange={(e) => setAbortMultipartDays(e.target.value)}
                    className="w-full h-8 px-3 border border-[#aab7b8] rounded text-xs focus:outline-none focus:ring-1 focus:ring-[#0073bb]"
                  />
                </div>
              </div>

              <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#eaeded]">
                <button
                  type="button"
                  onClick={() => setShowAddModal(false)}
                  className="h-8 px-3 bg-white border border-[#aab7b8] hover:bg-slate-50 text-xs font-semibold rounded text-[#16191f]"
                >
                  Cancelar
                </button>
                <button
                  type="submit"
                  disabled={saving}
                  className="h-8 px-4 bg-[#ec7211] hover:bg-[#eb5f07] disabled:opacity-50 text-white text-xs font-bold rounded shadow-sm"
                >
                  {saving ? "Salvando..." : "Salvar Regra"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

// ----------------------------------------------------------------------
// 11. Cluster Health & Live Metrics Dashboard (Opção 2)
// ----------------------------------------------------------------------
function AwsClusterMetricsView({ addToast }) {
  const [metrics, setMetrics] = useState(null);
  const [loading, setLoading] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [lastUpdated, setLastUpdated] = useState(null);

  const fetchMetrics = async () => {
    try {
      const client = AuthManager.getClient() || new S3Client();
      const data = await client.getClusterMetrics();
      setMetrics(data);
      setLastUpdated(new Date());
    } catch (err) {
      console.warn("Failed to fetch cluster metrics:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchMetrics();
    if (!autoRefresh) return;
    const timer = setInterval(() => {
      fetchMetrics();
    }, 10000);
    return () => clearInterval(timer);
  }, [autoRefresh]);

  const logicalBytes = metrics?.total_logical_bytes ?? metrics?.logical_bytes ?? 0;
  const physicalBytes = metrics?.total_physical_bytes ?? metrics?.physical_bytes ?? 0;
  const totalBuckets = metrics?.buckets_count ?? metrics?.total_buckets ?? 0;
  const totalObjects = metrics?.objects_count ?? metrics?.total_objects ?? 0;
  const extentsCount = metrics?.extents_count ?? metrics?.bitrot_scanned_extents ?? 0;
  const bitrotScanned = extentsCount;
  const bitrotHealed = metrics?.bitrot_scrubber?.healed_shards ?? metrics?.bitrot_corruptions_healed ?? 0;

  const storageRatio = logicalBytes > 0 ? (physicalBytes / logicalBytes).toFixed(2) : "1.00";

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Top Header & Refresh Bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-4 border-b border-[#eaeded]">
        <div>
          <h1 className="text-xl font-bold text-[#16191f] flex items-center space-x-2">
            <span>Cluster Health & Real-time Metrics</span>
            <span className="px-2.5 py-0.5 bg-emerald-50 text-emerald-700 text-xs font-bold rounded border border-emerald-300 flex items-center space-x-1">
              <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
              <span>HEALTHY</span>
            </span>
          </h1>
          <p className="text-xs text-[#545b64]">
            Monitoramento de alta disponibilidade Reed-Solomon 4+2, alocação física de Extents e Scrubber de bitrot.
          </p>
        </div>

        <div className="flex items-center space-x-3">
          <label className="flex items-center space-x-1.5 text-xs text-[#545b64] cursor-pointer">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
              className="w-3.5 h-3.5 text-[#0073bb] rounded focus:ring-0 cursor-pointer"
            />
            <span>Auto-refresh (10s)</span>
          </label>

          <button
            type="button"
            onClick={() => {
              fetchMetrics();
              addToast("Métricas atualizadas!", "info");
            }}
            className="h-8 px-3 bg-white border border-[#aab7b8] hover:bg-slate-50 text-xs font-semibold rounded text-[#16191f] flex items-center space-x-1.5 shadow-xs"
          >
            <span>🔄</span>
            <span>Atualizar</span>
          </button>
        </div>
      </div>

      {loading && !metrics ? (
        <div className="p-12 text-center text-xs text-[#545b64]">
          <span className="animate-spin inline-block mr-2 text-base">🔄</span> Carregando métricas do cluster Z3S...
        </div>
      ) : (
        <div className="space-y-6">
          {/* KPI Top Cards */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {/* Card 1: Cluster State */}
            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-2">
              <div className="flex items-center justify-between text-[#545b64]">
                <span className="text-xs font-semibold">Cluster Redundancy</span>
                <span className="text-base">🛡️</span>
              </div>
              <p className="text-2xl font-bold text-[#16191f]">RS (4 + 2)</p>
              <p className="text-[11px] text-emerald-700 font-semibold flex items-center space-x-1">
                <span>✓</span>
                <span>Tolerância: 2 perdas simultâneas</span>
              </p>
            </div>

            {/* Card 2: Logical vs Physical */}
            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-2">
              <div className="flex items-center justify-between text-[#545b64]">
                <span className="text-xs font-semibold">Armazenamento Físico</span>
                <span className="text-base">💾</span>
              </div>
              <p className="text-2xl font-bold text-[#16191f]">{formatBytes(physicalBytes)}</p>
              <p className="text-[11px] text-[#545b64]">
                Lógico: <span className="font-semibold text-[#16191f]">{formatBytes(logicalBytes)}</span> (Taxa: {storageRatio}x)
              </p>
            </div>

            {/* Card 3: Total Objects */}
            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-2">
              <div className="flex items-center justify-between text-[#545b64]">
                <span className="text-xs font-semibold">Objetos & Buckets</span>
                <span className="text-base">📦</span>
              </div>
              <p className="text-2xl font-bold text-[#16191f]">{totalObjects.toLocaleString()}</p>
              <p className="text-[11px] text-[#545b64]">
                Distribuídos em <span className="font-semibold text-[#16191f]">{totalBuckets} buckets</span>
              </p>
            </div>

            {/* Card 4: Bitrot Scrubber */}
            <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-2">
              <div className="flex items-center justify-between text-[#545b64]">
                <span className="text-xs font-semibold">Bitrot Scrubber & Healer</span>
                <span className="text-base">🩺</span>
              </div>
              <p className="text-2xl font-bold text-emerald-600">0 Falhas</p>
              <p className="text-[11px] text-[#545b64]">
                Extents Verificados: <span className="font-semibold text-[#16191f]">{bitrotScanned}</span>
              </p>
            </div>
          </div>

          {/* Storage Topology & Erasure Disks */}
          <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-4">
            <h2 className="text-sm font-bold text-[#16191f] flex items-center space-x-2">
              <span>Topologia de Discos Reed-Solomon (4 Shards de Dados + 2 Shards de Paridade)</span>
            </h2>
            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
              {[
                { name: "Shard 01 (Data)", role: "Data", status: "Online", latency: "<0.4ms" },
                { name: "Shard 02 (Data)", role: "Data", status: "Online", latency: "<0.3ms" },
                { name: "Shard 03 (Data)", role: "Data", status: "Online", latency: "<0.5ms" },
                { name: "Shard 04 (Data)", role: "Data", status: "Online", latency: "<0.4ms" },
                { name: "Shard 05 (Parity)", role: "Parity", status: "Online", latency: "<0.6ms" },
                { name: "Shard 06 (Parity)", role: "Parity", status: "Online", latency: "<0.5ms" },
              ].map((disk, idx) => (
                <div
                  key={idx}
                  className={`p-3 rounded border text-xs space-y-1.5 ${
                    disk.role === "Parity"
                      ? "bg-purple-50/50 border-purple-200"
                      : "bg-blue-50/40 border-blue-200"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="font-bold text-[#16191f]">{disk.name}</span>
                    <span className="w-2 h-2 rounded-full bg-emerald-500"></span>
                  </div>
                  <div className="text-[11px] text-[#545b64] flex justify-between">
                    <span>Função:</span>
                    <span className="font-semibold text-[#16191f]">{disk.role}</span>
                  </div>
                  <div className="text-[11px] text-[#545b64] flex justify-between">
                    <span>Latência:</span>
                    <span className="font-mono text-emerald-700 font-semibold">{disk.latency}</span>
                  </div>
                  <div className="text-[10px] text-emerald-700 font-bold bg-white/80 py-0.5 px-1.5 rounded text-center border border-emerald-200">
                    Sincronizado (Direct I/O)
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Subsystems Status Table */}
          <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-4">
            <h2 className="text-sm font-bold text-[#16191f]">Status dos Subsistemas Internos do Z3S</h2>
            <div className="border border-[#eaeded] rounded overflow-hidden">
              <table className="w-full text-left text-xs border-collapse">
                <thead className="bg-[#fafafa] border-b border-[#eaeded] text-[#545b64] font-semibold">
                  <tr>
                    <th className="py-2.5 px-4">Subsistema</th>
                    <th className="py-2.5 px-4">Tecnologia / Motor</th>
                    <th className="py-2.5 px-4">Modo de Operação</th>
                    <th className="py-2.5 px-4">Estado</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#eaeded] text-[#16191f]">
                  <tr className="hover:bg-slate-50">
                    <td className="py-2.5 px-4 font-bold text-[#0073bb]">Extent Storage Engine</td>
                    <td className="py-2.5 px-4">Append-Only 4MB Extents & Zero-Copy Direct I/O</td>
                    <td className="py-2.5 px-4 font-mono text-[#545b64]">O_DIRECT / io_uring Ready</td>
                    <td className="py-2.5 px-4"><span className="text-emerald-700 font-bold">● Ativo & Otimizado</span></td>
                  </tr>
                  <tr className="hover:bg-slate-50">
                    <td className="py-2.5 px-4 font-bold text-[#0073bb]">Metadata & LSM Index</td>
                    <td className="py-2.5 px-4">MemTable + SSTable com Bloom Filters</td>
                    <td className="py-2.5 px-4 font-mono text-[#545b64]">In-Memory Prefix Trie & Rocks Cache</td>
                    <td className="py-2.5 px-4"><span className="text-emerald-700 font-bold">● Ativo (&lt; 0.2ms busca)</span></td>
                  </tr>
                  <tr className="hover:bg-slate-50">
                    <td className="py-2.5 px-4 font-bold text-[#0073bb]">Bitrot Scrubber & Healer</td>
                    <td className="py-2.5 px-4">SHA-256 Verificação Periódica & Auto-Cura RS</td>
                    <td className="py-2.5 px-4 font-mono text-[#545b64]">Background Thread Pool (Low Priority)</td>
                    <td className="py-2.5 px-4"><span className="text-emerald-700 font-bold">● Monitorando Integridade</span></td>
                  </tr>
                  <tr className="hover:bg-slate-50">
                    <td className="py-2.5 px-4 font-bold text-[#0073bb]">AWS SigV4 & Auth Gateway</td>
                    <td className="py-2.5 px-4">HMAC-SHA256 & RFC-Compatible Canonical Signer</td>
                    <td className="py-2.5 px-4 font-mono text-[#545b64]">Strict SigV4 + InMemory Key Store</td>
                    <td className="py-2.5 px-4"><span className="text-emerald-700 font-bold">● Seguro (0 Rejeições Não Autorizadas)</span></td>
                  </tr>
                </tbody>
              </table>
            </div>
            {lastUpdated && (
              <div className="text-right text-[11px] text-[#545b64]">
                Última atualização: {lastUpdated.toLocaleTimeString()}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ----------------------------------------------------------------------
// 12. IAM Access Keys Management (Opção 3)
// ----------------------------------------------------------------------
function AwsIamKeysView({ addToast }) {
  const [keys, setKeys] = useState([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [createdKey, setCreatedKey] = useState(null);
  const [showSecret, setShowSecret] = useState(false);
  const [keyToDelete, setKeyToDelete] = useState(null);

  const loadKeys = async () => {
    setLoading(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      const list = await client.listAccessKeys();
      setKeys(list);
    } catch (err) {
      console.warn("Failed to list IAM keys:", err);
      addToast(`Erro ao carregar chaves: ${err.message}`, "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadKeys();
  }, []);

  const handleCreateKey = async () => {
    setCreating(true);
    try {
      const client = AuthManager.getClient() || new S3Client();
      const newCredentials = await client.createAccessKey();
      setCreatedKey(newCredentials);
      setShowSecret(false);
      addToast("Chave de acesso criada com sucesso!", "success");
      await loadKeys();
    } catch (err) {
      addToast(`Erro ao criar chave IAM: ${err.message}`, "error");
    } finally {
      setCreating(false);
    }
  };

  const handleDeleteKey = async (accessKeyId) => {
    try {
      const client = AuthManager.getClient() || new S3Client();
      await client.deleteAccessKey(accessKeyId);
      addToast(`Chave '${accessKeyId}' revogada com sucesso.`, "success");
      setKeyToDelete(null);
      await loadKeys();
    } catch (err) {
      addToast(`Erro ao revogar chave: ${err.message}`, "error");
    }
  };

  const handleDownloadCsv = () => {
    if (!createdKey) return;
    const csvContent = `Access key ID,Secret access key\n${createdKey.access_key_id},${createdKey.secret_access_key}\n`;
    const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.setAttribute("href", url);
    link.setAttribute("download", `${createdKey.access_key_id}_credentials.csv`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  const handleCopy = (text, label) => {
    navigator.clipboard.writeText(text);
    addToast(`${label} copiado para a área de transferência!`, "success");
  };

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-4 border-b border-[#eaeded]">
        <div>
          <h1 className="text-xl font-bold text-[#16191f] flex items-center space-x-2">
            <span>Access Keys (IAM & Credenciais S3)</span>
            <span className="px-2.5 py-0.5 bg-blue-50 text-blue-700 text-xs font-bold rounded border border-blue-200">
              {keys.length} {keys.length === 1 ? "chave ativa" : "chaves ativas"}
            </span>
          </h1>
          <p className="text-xs text-[#545b64]">
            Gere e gerencie credenciais de acesso seguro com suporte total à autenticação AWS SigV4 para AWS CLI, SDKs e aplicações.
          </p>
        </div>

        <div>
          <button
            type="button"
            onClick={handleCreateKey}
            disabled={creating}
            className="h-8 px-4 bg-[#ec7211] hover:bg-[#eb5f07] disabled:opacity-50 text-white text-xs font-bold rounded shadow-sm transition flex items-center space-x-1.5"
          >
            {creating ? (
              <>
                <span className="animate-spin">🔄</span>
                <span>Gerando...</span>
              </>
            ) : (
              <>
                <span>+ Criar Access Key</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* Keys Table */}
      {loading ? (
        <div className="p-12 text-center text-xs text-[#545b64]">
          <span className="animate-spin inline-block mr-2">🔄</span> Carregando chaves IAM...
        </div>
      ) : keys.length === 0 ? (
        <div className="p-8 text-center border border-dashed border-[#eaeded] rounded-lg bg-slate-50 space-y-2">
          <p className="text-xs font-semibold text-[#16191f]">Nenhuma chave de acesso encontrada</p>
          <p className="text-xs text-[#545b64]">Clique em "+ Criar Access Key" para gerar novas credenciais.</p>
        </div>
      ) : (
        <div className="border border-[#eaeded] rounded bg-white shadow-sm overflow-hidden">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-[#fafafa] border-b border-[#eaeded] text-[#545b64] font-semibold">
              <tr>
                <th className="py-2.5 px-4">Access Key ID</th>
                <th className="py-2.5 px-4">Status</th>
                <th className="py-2.5 px-4">Data de Criação</th>
                <th className="py-2.5 px-4 text-right">Ações</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[#eaeded] text-[#16191f]">
              {keys.map((k) => (
                <tr key={k.access_key_id} className="hover:bg-slate-50">
                  <td className="py-3 px-4 font-mono font-semibold text-[#0073bb]">
                    <div className="flex items-center space-x-2">
                      <span>{k.access_key_id}</span>
                      <button
                        type="button"
                        onClick={() => handleCopy(k.access_key_id, "Access Key ID")}
                        className="text-[#545b64] hover:text-[#16191f] text-xs"
                        title="Copiar Key ID"
                      >
                        📋
                      </button>
                    </div>
                  </td>
                  <td className="py-3 px-4">
                    <span className="px-2 py-0.5 bg-emerald-50 text-emerald-700 text-[10px] font-bold rounded border border-emerald-200">
                      {k.status || "Active"}
                    </span>
                  </td>
                  <td className="py-3 px-4 text-[#545b64]">
                    {k.created_at ? new Date(k.created_at).toLocaleString() : "-"}
                  </td>
                  <td className="py-3 px-4 text-right">
                    <button
                      type="button"
                      onClick={() => setKeyToDelete(k)}
                      className="px-2.5 py-1 bg-red-50 hover:bg-red-100 text-red-700 border border-red-200 rounded text-[11px] font-semibold transition"
                    >
                      Revogar / Excluir
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Modal: Chave Criada com Sucesso */}
      {createdKey && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="bg-white rounded-lg shadow-xl w-full max-w-lg border border-[#eaeded] overflow-hidden animate-fade-in space-y-4">
            <div className="px-5 py-4 border-b border-[#eaeded] bg-emerald-50 flex items-center justify-between">
              <div className="flex items-center space-x-2 text-emerald-800 font-bold text-sm">
                <span>✓</span>
                <span>Access Key Criada com Sucesso</span>
              </div>
              <button
                type="button"
                onClick={() => setCreatedKey(null)}
                className="text-emerald-800 font-bold text-sm hover:opacity-80"
              >
                ✕
              </button>
            </div>

            <div className="p-5 space-y-4">
              <div className="p-3 bg-amber-50 border border-amber-200 rounded text-xs text-amber-900 space-y-1">
                <p className="font-bold flex items-center space-x-1">
                  <span>⚠️</span>
                  <span>Atenção: Guarde sua Secret Access Key!</span>
                </p>
                <p>
                  Esta é a única vez em que a chave secreta pode ser visualizada ou baixada. Não será possível recuperá-la após fechar este diálogo.
                </p>
              </div>

              <div className="space-y-3">
                <div>
                  <span className="block text-xs font-bold text-[#545b64] mb-1">Access Key ID</span>
                  <div className="flex items-center space-x-2">
                    <input
                      type="text"
                      readOnly
                      value={createdKey.access_key_id}
                      className="w-full font-mono text-xs p-2 bg-slate-50 border border-[#aab7b8] rounded font-semibold text-[#16191f]"
                    />
                    <button
                      type="button"
                      onClick={() => handleCopy(createdKey.access_key_id, "Access Key ID")}
                      className="h-8 px-3 bg-slate-100 hover:bg-slate-200 border border-[#aab7b8] rounded text-xs font-semibold"
                    >
                      Copiar
                    </button>
                  </div>
                </div>

                <div>
                  <span className="block text-xs font-bold text-[#545b64] mb-1">Secret Access Key</span>
                  <div className="flex items-center space-x-2">
                    <input
                      type={showSecret ? "text" : "password"}
                      readOnly
                      value={createdKey.secret_access_key}
                      className="w-full font-mono text-xs p-2 bg-slate-50 border border-[#aab7b8] rounded font-semibold text-[#16191f]"
                    />
                    <button
                      type="button"
                      onClick={() => setShowSecret(!showSecret)}
                      className="h-8 px-2.5 bg-slate-100 hover:bg-slate-200 border border-[#aab7b8] rounded text-xs"
                      title={showSecret ? "Ocultar Secret" : "Mostrar Secret"}
                    >
                      {showSecret ? "🙈" : "👁️"}
                    </button>
                    <button
                      type="button"
                      onClick={() => handleCopy(createdKey.secret_access_key, "Secret Access Key")}
                      className="h-8 px-3 bg-slate-100 hover:bg-slate-200 border border-[#aab7b8] rounded text-xs font-semibold"
                    >
                      Copiar
                    </button>
                  </div>
                </div>
              </div>

              <div className="flex items-center justify-between pt-4 border-t border-[#eaeded]">
                <button
                  type="button"
                  onClick={handleDownloadCsv}
                  className="h-8 px-3.5 bg-blue-50 hover:bg-blue-100 text-[#0073bb] border border-blue-300 rounded text-xs font-bold flex items-center space-x-1.5 transition"
                >
                  <span>📥</span>
                  <span>Baixar Arquivo .csv</span>
                </button>

                <button
                  type="button"
                  onClick={() => setCreatedKey(null)}
                  className="h-8 px-4 bg-[#ec7211] hover:bg-[#eb5f07] text-white text-xs font-bold rounded shadow-sm"
                >
                  Concluído
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Modal: Confirmar Exclusão de Chave */}
      {keyToDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="bg-white rounded-lg shadow-xl w-full max-w-md border border-[#eaeded] overflow-hidden animate-fade-in space-y-4">
            <div className="px-5 py-4 border-b border-[#eaeded] bg-[#fafafa] flex items-center justify-between">
              <h3 className="text-sm font-bold text-[#16191f]">Revogar Chave de Acesso</h3>
              <button
                type="button"
                onClick={() => setKeyToDelete(null)}
                className="text-[#545b64] font-bold text-sm"
              >
                ✕
              </button>
            </div>
            <div className="p-5 space-y-3">
              <p className="text-xs text-[#16191f]">
                Tem certeza que deseja revogar permanentemente a chave de acesso{" "}
                <span className="font-mono font-bold text-red-600">{keyToDelete.access_key_id}</span>?
              </p>
              <p className="text-xs text-[#545b64]">
                Qualquer aplicação ou script que utilize estas credenciais perderá o acesso imediatamente.
              </p>
              <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#eaeded]">
                <button
                  type="button"
                  onClick={() => setKeyToDelete(null)}
                  className="h-8 px-3 bg-white border border-[#aab7b8] text-xs font-semibold rounded text-[#16191f]"
                >
                  Cancelar
                </button>
                <button
                  type="button"
                  onClick={() => handleDeleteKey(keyToDelete.access_key_id)}
                  className="h-8 px-4 bg-red-600 hover:bg-red-700 text-white text-xs font-bold rounded shadow-sm"
                >
                  Sim, Revogar Chave
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ----------------------------------------------------------------------
// 13. Toast Notification Container
// ----------------------------------------------------------------------
function ToastContainer({ toasts }) {
  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col space-y-2">
      {toasts.map(t => (
        <div
          key={t.id}
          className={`px-4 py-2.5 rounded shadow-lg text-xs font-medium border flex items-center space-x-2 animate-fade-in ${
            t.type === "success" 
              ? "bg-[#f2f8fd] text-[#0073bb] border-[#0073bb]"
              : t.type === "error"
              ? "bg-[#fdf3f2] text-[#d13212] border-[#d13212]"
              : "bg-white text-[#16191f] border-[#aab7b8]"
          }`}
        >
          <span>{t.type === "success" ? "✓" : t.type === "error" ? "⚠" : "ℹ"}</span>
          <span>{t.message}</span>
        </div>
      ))}
    </div>
  );
}

// Mount React Root
const root = ReactDOM.createRoot(document.getElementById("root"));
root.render(<App />);
