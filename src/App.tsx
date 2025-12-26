import { useEffect } from "react";
import { useProjectStore } from "./stores/projectStore";
import { usePlayerStore } from "./stores/playerStore";
import { useKeyboard } from "./hooks/useKeyboard";
import { usePlayerSync } from "./hooks/usePlayerSync";
import { PlayView } from "./components/views/PlayView";
import { EditView } from "./components/views/EditView";
import { BrightnessPanel } from "./components/player/BrightnessPanel";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";
import { Play, Settings } from "lucide-react";
import "./App.css";

function App() {
  const { project, newProject } = useProjectStore();
  const { status, error } = usePlayerStore();

  // キーボードショートカット
  useKeyboard();

  // プレイヤー状態同期
  usePlayerSync();

  // 初期プロジェクト作成
  useEffect(() => {
    if (!project) {
      newProject("New Project");
    }
  }, [project, newProject]);

  return (
    <div className="h-screen flex flex-col bg-background text-foreground">
      <Tabs defaultValue="play" className="flex flex-col h-full">
        {/* Header with Tabs */}
        <header className="border-b px-4 py-2 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <h1 className="text-lg font-semibold">
              {project?.name || "TauriLivePlayer"}
            </h1>
            <TabsList>
              <TabsTrigger value="play" className="gap-2">
                <Play className="w-4 h-4" />
                Play
              </TabsTrigger>
              <TabsTrigger value="edit" className="gap-2">
                <Settings className="w-4 h-4" />
                Edit
              </TabsTrigger>
            </TabsList>
          </div>
          <div className="flex items-center gap-4">
            {/* Brightness (compact) */}
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground">Brightness:</span>
              <BrightnessPanel compact />
            </div>
            {/* Status */}
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <div
                className={`w-2 h-2 rounded-full ${
                  status === "playing"
                    ? "bg-green-500"
                    : status === "paused"
                      ? "bg-yellow-500"
                      : status === "error"
                        ? "bg-red-500"
                        : "bg-gray-400"
                }`}
              />
              <span className="capitalize">{status}</span>
              {error && <span className="text-destructive">{error}</span>}
            </div>
          </div>
        </header>

        {/* Main Content */}
        <TabsContent value="play" className="flex-1 overflow-hidden m-0">
          <PlayView />
        </TabsContent>

        <TabsContent value="edit" className="flex-1 overflow-hidden m-0">
          <EditView />
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default App;
