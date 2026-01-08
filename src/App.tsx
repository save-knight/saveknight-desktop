import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { Shield, Folder, Upload, RefreshCw, Check, X, Loader2, LogOut, ExternalLink } from 'lucide-react';

interface DetectedGame {
  name: string;
  paths: DetectedSavePath[];
  total_size_bytes: number;
  last_modified: string | null;
}

interface DetectedSavePath {
  pattern: string;
  resolved_path: string;
  exists: boolean;
  file_count: number;
  total_size_bytes: number;
}

interface AuthStatus {
  is_authenticated: boolean;
  device_id: string | null;
  user_email: string | null;
  plan_name: string | null;
}

interface GameProfile {
  id: string;
  name: string;
  platform: string;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export default function App() {
  const [authStatus, setAuthStatus] = useState<AuthStatus | null>(null);
  const [detectedGames, setDetectedGames] = useState<DetectedGame[]>([]);
  const [selectedGames, setSelectedGames] = useState<Set<string>>(new Set());
  const [gameProfiles, setGameProfiles] = useState<GameProfile[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [activeTab, setActiveTab] = useState<'games' | 'settings'>('games');
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  
  const [sessionCookie, setSessionCookie] = useState('');
  const [deviceName, setDeviceName] = useState('');
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [showLoginForm, setShowLoginForm] = useState(false);

  useEffect(() => {
    checkAuth();
    
    const unlisten = listen('trigger-scan', () => {
      handleScan();
    });

    const hostname = window.navigator.userAgent.includes('Windows') ? 'Windows PC' : 'Desktop';
    setDeviceName(hostname);

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function checkAuth() {
    try {
      const status = await invoke<AuthStatus>('get_auth_status');
      setAuthStatus(status);
      if (status.is_authenticated) {
        loadGameProfiles();
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function loadGameProfiles() {
    try {
      const profiles = await invoke<GameProfile[]>('get_game_profiles');
      setGameProfiles(profiles);
    } catch (e) {
      console.error('Failed to load game profiles:', e);
    }
  }

  async function handleLogin() {
    if (!sessionCookie.trim()) {
      setError('Please enter your session cookie');
      return;
    }
    if (!deviceName.trim()) {
      setError('Please enter a device name');
      return;
    }

    setIsLoggingIn(true);
    setError(null);
    try {
      const status = await invoke<AuthStatus>('login', {
        sessionCookie: sessionCookie.trim(),
        deviceName: deviceName.trim(),
      });
      setAuthStatus(status);
      setSessionCookie('');
      setShowLoginForm(false);
      if (status.is_authenticated) {
        loadGameProfiles();
        setSuccessMessage('Successfully connected to SaveKnight!');
        setTimeout(() => setSuccessMessage(null), 3000);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoggingIn(false);
    }
  }

  async function handleLogout() {
    try {
      await invoke('logout');
      setAuthStatus({ is_authenticated: false, device_id: null, user_email: null, plan_name: null });
      setDetectedGames([]);
      setSelectedGames(new Set());
      setGameProfiles([]);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleScan() {
    setIsScanning(true);
    setError(null);
    try {
      const games = await invoke<DetectedGame[]>('scan_games');
      setDetectedGames(games);
      if (games.length > 0) {
        setSuccessMessage(`Found ${games.length} games with save files!`);
        setTimeout(() => setSuccessMessage(null), 3000);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsScanning(false);
    }
  }

  async function handleUpload() {
    if (selectedGames.size === 0) {
      setError('Please select at least one game to backup');
      return;
    }

    setIsUploading(true);
    setError(null);
    let successCount = 0;
    let failCount = 0;

    try {
      const gamesToUpload = detectedGames.filter((g) => selectedGames.has(g.name));
      
      for (const game of gamesToUpload) {
        let profile = gameProfiles.find((p) => p.name.toLowerCase() === game.name.toLowerCase());
        
        if (!profile) {
          try {
            const newProfile = await invoke<GameProfile>('create_game_profile', {
              name: game.name,
              platform: 'PC',
            });
            profile = newProfile;
            setGameProfiles((prev) => [...prev, newProfile]);
          } catch (e) {
            console.error(`Failed to create profile for ${game.name}:`, e);
            failCount++;
            continue;
          }
        }
        
        if (profile) {
          try {
            await invoke('upload_saves', {
              games: [game],
              gameProfileId: profile.id,
            });
            successCount++;
          } catch (e) {
            console.error(`Failed to upload ${game.name}:`, e);
            failCount++;
          }
        }
      }
      
      setSelectedGames(new Set());
      
      if (successCount > 0) {
        setSuccessMessage(`Successfully backed up ${successCount} game(s)!`);
        setTimeout(() => setSuccessMessage(null), 5000);
      }
      if (failCount > 0) {
        setError(`Failed to backup ${failCount} game(s). Check logs for details.`);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsUploading(false);
    }
  }

  function toggleGameSelection(gameName: string) {
    setSelectedGames((prev) => {
      const next = new Set(prev);
      if (next.has(gameName)) {
        next.delete(gameName);
      } else {
        next.add(gameName);
      }
      return next;
    });
  }

  function selectAll() {
    setSelectedGames(new Set(detectedGames.map((g) => g.name)));
  }

  function deselectAll() {
    setSelectedGames(new Set());
  }

  if (!authStatus?.is_authenticated) {
    return (
      <div className="flex flex-col items-center justify-center min-h-screen p-8">
        <Shield className="w-16 h-16 text-primary mb-4" />
        <h1 className="text-2xl font-bold mb-2">SaveKnight Desktop</h1>
        <p className="text-muted-foreground mb-6 text-center max-w-md">
          Connect your SaveKnight account to start backing up your game saves automatically.
        </p>
        
        {error && (
          <div className="mb-4 p-3 bg-destructive/10 text-destructive rounded-md flex items-center gap-2 max-w-md w-full">
            <X className="w-4 h-4 flex-shrink-0" />
            <span className="text-sm">{error}</span>
          </div>
        )}

        {!showLoginForm ? (
          <div className="space-y-4 text-center">
            <button
              onClick={() => setShowLoginForm(true)}
              className="px-6 py-3 bg-primary text-primary-foreground rounded-md font-medium hover:bg-primary/90"
            >
              Connect Account
            </button>
            <p className="text-sm text-muted-foreground">
              You'll need to sign in on the SaveKnight website first.
            </p>
          </div>
        ) : (
          <div className="space-y-4 w-full max-w-md">
            <div className="p-4 border rounded-lg bg-muted/30">
              <h3 className="font-medium mb-2 flex items-center gap-2">
                <span>Step 1: Sign in to SaveKnight</span>
              </h3>
              <a
                href="https://saveknight.com"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-2 text-primary hover:underline text-sm"
              >
                Open SaveKnight website
                <ExternalLink className="w-3 h-3" />
              </a>
            </div>

            <div className="p-4 border rounded-lg bg-muted/30">
              <h3 className="font-medium mb-2">Step 2: Copy your session cookie</h3>
              <p className="text-sm text-muted-foreground mb-2">
                After signing in, open browser DevTools (F12), go to Application {'>'} Cookies, 
                and copy the value of "connect.sid".
              </p>
            </div>

            <div className="space-y-3">
              <div>
                <label className="block text-sm font-medium mb-1">Device Name</label>
                <input
                  type="text"
                  value={deviceName}
                  onChange={(e) => setDeviceName(e.target.value)}
                  placeholder="My Gaming PC"
                  className="w-full px-3 py-2 border rounded-md bg-background"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">Session Cookie</label>
                <input
                  type="password"
                  value={sessionCookie}
                  onChange={(e) => setSessionCookie(e.target.value)}
                  placeholder="Paste connect.sid value here"
                  className="w-full px-3 py-2 border rounded-md bg-background font-mono text-sm"
                />
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => {
                    setShowLoginForm(false);
                    setError(null);
                  }}
                  className="flex-1 px-4 py-2 border rounded-md hover:bg-muted"
                >
                  Cancel
                </button>
                <button
                  onClick={handleLogin}
                  disabled={isLoggingIn}
                  className="flex-1 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 flex items-center justify-center gap-2"
                >
                  {isLoggingIn ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      Connecting...
                    </>
                  ) : (
                    'Connect'
                  )}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="flex flex-col min-h-screen">
      <header className="flex items-center justify-between p-4 border-b">
        <div className="flex items-center gap-2">
          <Shield className="w-6 h-6 text-primary" />
          <span className="font-bold text-lg">SaveKnight</span>
        </div>
        <div className="flex items-center gap-4">
          <span className="text-sm text-muted-foreground">
            {authStatus.user_email} ({authStatus.plan_name})
          </span>
          <nav className="flex gap-2">
            <button
              onClick={() => setActiveTab('games')}
              className={`px-3 py-1.5 rounded-md text-sm ${
                activeTab === 'games'
                  ? 'bg-primary text-primary-foreground'
                  : 'hover:bg-muted'
              }`}
            >
              Games
            </button>
            <button
              onClick={() => setActiveTab('settings')}
              className={`px-3 py-1.5 rounded-md text-sm ${
                activeTab === 'settings'
                  ? 'bg-primary text-primary-foreground'
                  : 'hover:bg-muted'
              }`}
            >
              Settings
            </button>
          </nav>
        </div>
      </header>

      <main className="flex-1 p-6">
        {error && (
          <div className="mb-4 p-3 bg-destructive/10 text-destructive rounded-md flex items-center gap-2">
            <X className="w-4 h-4" />
            {error}
            <button onClick={() => setError(null)} className="ml-auto hover:opacity-70">
              <X className="w-4 h-4" />
            </button>
          </div>
        )}

        {successMessage && (
          <div className="mb-4 p-3 bg-green-500/10 text-green-600 dark:text-green-400 rounded-md flex items-center gap-2">
            <Check className="w-4 h-4" />
            {successMessage}
          </div>
        )}

        {activeTab === 'games' && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-semibold">Detected Games</h2>
              <div className="flex gap-2">
                <button
                  onClick={handleScan}
                  disabled={isScanning}
                  className="flex items-center gap-2 px-4 py-2 bg-secondary text-secondary-foreground rounded-md hover:bg-secondary/80 disabled:opacity-50"
                >
                  {isScanning ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <RefreshCw className="w-4 h-4" />
                  )}
                  Scan
                </button>
                <button
                  onClick={handleUpload}
                  disabled={isUploading || selectedGames.size === 0}
                  className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50"
                >
                  {isUploading ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Upload className="w-4 h-4" />
                  )}
                  Backup Selected ({selectedGames.size})
                </button>
              </div>
            </div>

            {detectedGames.length === 0 ? (
              <div className="text-center py-12 text-muted-foreground">
                <Folder className="w-12 h-12 mx-auto mb-4 opacity-50" />
                <p>No games detected yet. Click "Scan" to search for game saves.</p>
                <p className="text-sm mt-2">
                  SaveKnight scans common save locations for over 10,000 games.
                </p>
              </div>
            ) : (
              <>
                <div className="flex items-center gap-4 text-sm">
                  <span className="text-muted-foreground">
                    {detectedGames.length} games found
                  </span>
                  <button onClick={selectAll} className="text-primary hover:underline">
                    Select All
                  </button>
                  <button onClick={deselectAll} className="text-primary hover:underline">
                    Deselect All
                  </button>
                </div>

                <div className="grid gap-3">
                  {detectedGames.map((game) => (
                    <div
                      key={game.name}
                      onClick={() => toggleGameSelection(game.name)}
                      className={`p-4 border rounded-lg cursor-pointer transition-colors ${
                        selectedGames.has(game.name)
                          ? 'border-primary bg-primary/5'
                          : 'hover:bg-muted/50'
                      }`}
                    >
                      <div className="flex items-start justify-between">
                        <div className="flex items-start gap-3">
                          <div
                            className={`mt-1 w-5 h-5 rounded border flex items-center justify-center ${
                              selectedGames.has(game.name)
                                ? 'bg-primary border-primary'
                                : 'border-input'
                            }`}
                          >
                            {selectedGames.has(game.name) && (
                              <Check className="w-3 h-3 text-primary-foreground" />
                            )}
                          </div>
                          <div>
                            <h3 className="font-medium">{game.name}</h3>
                            <p className="text-sm text-muted-foreground">
                              {game.paths.filter((p) => p.exists).length} save location(s) found
                            </p>
                          </div>
                        </div>
                        <div className="text-right text-sm">
                          <div className="font-medium">{formatBytes(game.total_size_bytes)}</div>
                          {game.last_modified && (
                            <div className="text-muted-foreground">
                              Last modified: {game.last_modified}
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
        )}

        {activeTab === 'settings' && (
          <div className="max-w-2xl space-y-6">
            <h2 className="text-xl font-semibold">Settings</h2>
            
            <div className="space-y-4">
              <div className="p-4 border rounded-lg">
                <h3 className="font-medium mb-2">Account</h3>
                <p className="text-sm text-muted-foreground mb-2">
                  Signed in as {authStatus.user_email}
                </p>
                <p className="text-sm text-muted-foreground mb-4">
                  Plan: {authStatus.plan_name}
                </p>
                <button
                  onClick={handleLogout}
                  className="flex items-center gap-2 px-4 py-2 border border-destructive text-destructive rounded-md hover:bg-destructive/10"
                >
                  <LogOut className="w-4 h-4" />
                  Sign Out
                </button>
              </div>

              <div className="p-4 border rounded-lg">
                <h3 className="font-medium mb-2">Synced Games</h3>
                <p className="text-sm text-muted-foreground mb-2">
                  {gameProfiles.length} game profiles connected
                </p>
                {gameProfiles.length > 0 && (
                  <ul className="text-sm space-y-1">
                    {gameProfiles.slice(0, 5).map((p) => (
                      <li key={p.id} className="text-muted-foreground">
                        {p.name}
                      </li>
                    ))}
                    {gameProfiles.length > 5 && (
                      <li className="text-muted-foreground">
                        ...and {gameProfiles.length - 5} more
                      </li>
                    )}
                  </ul>
                )}
              </div>

              <div className="p-4 border rounded-lg">
                <h3 className="font-medium mb-2">About</h3>
                <p className="text-sm text-muted-foreground">
                  SaveKnight Desktop v1.0.0
                </p>
                <p className="text-sm text-muted-foreground">
                  Open source under MIT License
                </p>
                <a
                  href="https://github.com/save-knight/saveknight-desktop"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-primary hover:underline"
                >
                  View on GitHub
                </a>
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}
