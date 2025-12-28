import { useEffect } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "./stores/projectStore";
import { usePlayerStore } from "./stores/playerStore";
import { useKeyboard } from "./hooks/useKeyboard";
import { usePlayerSync } from "./hooks/usePlayerSync";
import { PlayView } from "./components/views/PlayView";
import { EditView } from "./components/views/EditView";
import { BrightnessPanel } from "./components/player/BrightnessPanel";
import { VolumePanel } from "./components/player/VolumePanel";
import { Button } from "./components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";
import { Play, Settings, FolderOpen, Save } from "lucide-react";
import "./App.css";

function App() {
  const {
    project,
    newProject,
    loadProject,
    saveProject,
    isDirty,
    projectPath,
  } = useProjectStore();
  const { status, error } = usePlayerStore();

  // キーボードショートカット
  useKeyboard();

  // プレイヤー状態同期
  usePlayerSync();

  // 起動時に最後のプロジェクトを開く、なければ新規作成
  useEffect(() => {
    if (!project) {
      const lastPath = localStorage.getItem("lastProjectPath");
      if (lastPath) {
        loadProject(lastPath).catch((e) => {
          console.warn("Failed to load last project:", e);
          newProject("New Project");
        });
      } else {
        newProject("New Project");
      }
    }
  }, [project, loadProject, newProject]);

  const handleOpen = async () => {
    const path = await open({
      filters: [{ name: "Project", extensions: ["json"] }],
    });
    if (path) {
      await loadProject(path);
    }
  };

  const handleSave = async () => {
    if (projectPath) {
      await saveProject(projectPath);
    } else {
      await handleSaveAs();
    }
  };

  const handleSaveAs = async () => {
    const path = await save({
      filters: [{ name: "Project", extensions: ["json"] }],
      defaultPath: `${project?.name || "project"}.json`,
    });
    if (path) {
      await saveProject(path);
    }
  };

  return (
    <div className="h-screen flex flex-col bg-background text-foreground">
      <Tabs defaultValue="play" className="flex flex-col h-full">
        {/* Header with Tabs */}
        <header className="border-b px-4 py-2 flex items-center justify-between">
          <div className="flex items-center gap-4">
            {/* File operations */}
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="icon"
                onClick={handleOpen}
                title="Open Project"
              >
                <FolderOpen className="w-4 h-4" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleSave}
                title="Save Project"
              >
                <Save className="w-4 h-4" />
              </Button>
            </div>
            <h1 className="text-lg font-semibold">
              {project?.name || "TauriLivePlayer"}
              {isDirty && <span className="text-muted-foreground ml-1">*</span>}
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
            {/* Brightness & Volume (compact) */}
            <div className="flex items-center gap-4">
              <BrightnessPanel compact />
              <VolumePanel compact />
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
