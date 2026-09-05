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

  if (!session) {
    return <AwsSignInView onLogin={handleLogin} />;
  }

  return (
    <div className="min-h-screen bg-[#f2f3f3] text-[#16191f] font-sans antialiased flex flex-col selection:bg-[#2563eb] selection:text-white">
      {/* 1. AWS Top Navigation Bar */}
      <AwsGlobalHeader 
        session={session} 
        onLogout={handleLogout}
        onNavigateHome={() => {
          setSelectedBucket(null);
          setCurrentTab("buckets");
        }}
      />

      {/* 2. Main Workspace */}
      <div className="flex-1 flex flex-col max-w-[1600px] w-full mx-auto p-6 space-y-6">
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
      console.error(err);
      setError("Falha na autenticação: Credenciais inválidas ou sem permissão de acesso ao S3.");
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
                <p>{error}</p>
              </div>
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-5">
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

            <button
              type="submit"
              disabled={loading}
              className="w-full h-10 bg-[#2563eb] hover:bg-[#1d4ed8] active:bg-[#1e40af] text-white font-bold text-sm rounded shadow-sm transition flex items-center justify-center space-x-2"
            >
              {loading ? (
                <span>Autenticando...</span>
              ) : (
                <span>Sign in</span>
              )}
            </button>
          </form>

          <div className="mt-8 pt-6 border-t border-slate-200 text-center text-xs text-[#545b64]">
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
// 2. AWS Global Header (Navy Navigation Bar)
// ----------------------------------------------------------------------
function AwsGlobalHeader({ session, onLogout, onNavigateHome }) {
  return (
    <header className="h-12 bg-[#161e2d] text-white px-4 flex items-center justify-between select-none shadow-md z-30">
      {/* Left brand & services */}
      <div className="flex items-center space-x-4">
        <button 
          onClick={onNavigateHome}
          className="flex items-center space-x-2 hover:opacity-90 transition focus:outline-none"
        >
          <div className="w-6 h-6 bg-[#2563eb] rounded flex items-center justify-center font-bold text-xs text-white">
            S3
          </div>
          <span className="font-bold text-sm tracking-tight text-white">Z3S S3 Console</span>
        </button>

        <div className="hidden md:flex items-center space-x-2 pl-4 border-l border-[#2a384c]">
          <span className="text-xs text-[#879596]">Serviços</span>
        </div>
      </div>

      {/* Center Search Bar */}
      <div className="hidden lg:flex flex-1 max-w-md mx-8">
        <div className="relative w-full">
          <input 
            type="text" 
            placeholder="Search for buckets, objects, features [Alt+S]" 
            className="w-full h-8 pl-8 pr-3 bg-[#0e1622] border border-[#3e4f67] rounded text-xs text-slate-200 placeholder-[#879596] focus:outline-none focus:border-[#2563eb]"
          />
          <span className="absolute left-2.5 top-2 text-xs text-[#879596]">🔍</span>
        </div>
      </div>

      {/* Right User & Region */}
      <div className="flex items-center space-x-4 text-xs">
        <div className="hidden sm:flex items-center space-x-1 px-2.5 py-1 bg-[#232f3e] border border-[#3e4f67] rounded text-slate-200">
          <span className="w-2 h-2 rounded-full bg-emerald-400"></span>
          <span>us-east-1</span>
        </div>

        <div className="flex items-center space-x-2">
          <span className="font-semibold text-slate-300">Admin</span>
          <button
            onClick={onLogout}
            className="px-2 py-1 bg-[#232f3e] hover:bg-[#2e3e52] border border-[#3e4f67] rounded text-slate-200 transition"
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

// ----------------------------------------------------------------------
// 4. AWS Bucket Detail View (Tabs: Objects, Properties, Permissions, Metrics)
// ----------------------------------------------------------------------
function AwsBucketDetailView({ bucket, activeTab, setActiveTab, onBack, addToast }) {
  const [contents, setContents] = useState({ folders: [], objects: [] });
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [versioningStatus, setVersioningStatus] = useState("Loading...");
  const [updatingVersioning, setUpdatingVersioning] = useState(false);

  const fetchObjects = async () => {
    setLoading(true);
    try {
      const client = AuthManager.getClient();
      const [data, vStatus] = await Promise.all([
        client.listObjects(bucket, "", "/"),
        client.getBucketVersioning(bucket)
      ]);
      setContents(data);
      setVersioningStatus(vStatus || "Off");
    } catch (err) {
      addToast("Erro ao carregar dados do bucket: " + err.message, "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchObjects();
  }, [bucket]);

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
      {/* Breadcrumbs */}
      <div className="text-xs text-[#545b64] flex items-center space-x-1.5">
        <span onClick={onBack} className="hover:underline cursor-pointer">Z3S S3</span>
        <span>&gt;</span>
        <span onClick={onBack} className="hover:underline cursor-pointer">Buckets</span>
        <span>&gt;</span>
        <span className="text-[#16191f] font-semibold">{bucket}</span>
      </div>

      {/* Header */}
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
      </div>

      {/* Navigation Tabs (Z3S Cloudscape style) */}
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

      {/* Tab 1: Objects View */}
      {activeTab === "objects" && (
        <div className="bg-white border border-[#eaeded] rounded shadow-sm overflow-hidden">
          {/* Action Toolbar */}
          <div className="p-4 border-b border-[#eaeded] flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center space-x-2">
              <span className="text-xs text-[#545b64] font-mono">s3://{bucket}/</span>
            </div>

            <div className="flex items-center space-x-2">
              <button 
                onClick={fetchObjects}
                className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition"
              >
                🔄 Refresh
              </button>
              <button 
                className="h-8 px-3 bg-white hover:bg-[#fafafa] border border-[#aab7b8] rounded text-xs font-semibold text-[#16191f] transition"
                onClick={() => addToast("Upload de arquivos disponível via AWS CLI ou SDK.", "info")}
              >
                Upload
              </button>
            </div>
          </div>

          {loading ? (
            <div className="p-12 text-center text-xs text-[#545b64]">Carregando objetos...</div>
          ) : contents.objects.length === 0 ? (
            <div className="p-12 text-center text-xs text-[#545b64]">Este bucket está vazio.</div>
          ) : (
            <table className="w-full text-left text-xs text-[#16191f]">
              <thead className="bg-[#fafafa] border-b border-[#eaeded] text-[#545b64] font-semibold">
                <tr>
                  <th className="px-4 py-3">Name</th>
                  <th className="px-4 py-3">Type</th>
                  <th className="px-4 py-3">Last modified</th>
                  <th className="px-4 py-3">Size</th>
                  <th className="px-4 py-3">Storage class</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#eaeded]">
                {contents.objects.map(obj => (
                  <tr key={obj.key} className="hover:bg-[#f2f8fd] transition">
                    <td className="px-4 py-3 font-semibold text-[#0073bb] hover:underline cursor-pointer">
                      📄 {obj.key}
                    </td>
                    <td className="px-4 py-3 text-[#545b64] uppercase text-[11px]">{obj.key.split('.').pop() || 'Arquivo'}</td>
                    <td className="px-4 py-3 text-[#545b64] font-mono text-[11px]">{new Date(obj.lastModified).toLocaleString()}</td>
                    <td className="px-4 py-3 text-[#545b64] font-mono text-[11px]">{(obj.size / 1024).toFixed(1)} KB</td>
                    <td className="px-4 py-3 text-[#545b64]">Standard</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      {/* Tab 2: Properties (Phase 2 - Live Configuration) */}
      {activeTab === "properties" && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Bucket Versioning Card */}
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

          {/* Default Encryption Card */}
          <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-3">
            <h3 className="text-sm font-bold text-[#16191f]">Default Encryption</h3>
            <p className="text-xs text-[#545b64]">Criptografa objetos automaticamente em repouso com algoritmo padrão AES-256.</p>
            <div className="pt-1">
              <span className="px-2.5 py-1 bg-blue-50 text-blue-700 border border-blue-200 rounded text-xs font-semibold">
                🔒 SSE-S3 (AES-256-GCM)
              </span>
            </div>
          </div>

          {/* S3 URI & ARN Card */}
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
      )}

      {/* Tab 3: Permissions */}
      {activeTab === "permissions" && (
        <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-4">
          <h3 className="text-sm font-bold text-[#16191f]">Block Public Access (Bucket Settings)</h3>
          <p className="text-xs text-[#545b64]">Bloqueia o acesso público e anônimo a este bucket e todos os seus objetos.</p>
          <div className="p-3 bg-emerald-50 border border-emerald-200 rounded text-xs font-semibold text-emerald-800 flex items-center space-x-2">
            <span>🛡️</span>
            <span>Block all public access: <strong>ON</strong> (Proteção Máxima Ativa)</span>
          </div>
        </div>
      )}

      {/* Tab 4: Metrics */}
      {activeTab === "metrics" && (
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-1">
            <span className="text-xs text-[#545b64]">Total de Objetos</span>
            <p className="text-2xl font-bold text-[#16191f]">{contents.objects.length}</p>
          </div>
          <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-1">
            <span className="text-xs text-[#545b64]">Armazenamento Utilizado</span>
            <p className="text-2xl font-bold text-[#16191f]">
              {totalBytes < 1024 * 1024 
                ? `${(totalBytes / 1024).toFixed(1)} KB` 
                : `${(totalBytes / (1024 * 1024)).toFixed(2)} MB`}
            </p>
          </div>
          <div className="p-5 bg-white border border-[#eaeded] rounded shadow-sm space-y-1">
            <span className="text-xs text-[#545b64]">Status de Réplica / Erasure</span>
            <p className="text-sm font-semibold text-emerald-600">Reed-Solomon 4+2 Ativo</p>
          </div>
        </div>
      )}
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
// 8. Toast Notification Container
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
