import { useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useProjectStore } from "./stores/projectStore";
import { usePlayerStore } from "./stores/playerStore";
import { useKeyboard } from "./hooks/useKeyboard";
import { usePlayerSync } from "./hooks/usePlayerSync";
import { CueList } from "./components/cue/CueList";
import { PlayerControls } from "./components/player/PlayerControls";
import { BrightnessPanel } from "./components/player/BrightnessPanel";
import { Button } from "./components/ui/button";
import { FileVideo } from "lucide-react";
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

  // テスト用: 動画ファイルを開いて再生
  const handleOpenVideo = async () => {
    try {
      const file = await open({
        multiple: false,
        filters: [
          {
            name: "Video",
            extensions: ["mp4", "mov", "avi", "mkv", "webm"],
          },
        ],
      });

      if (file) {
        console.log("Opening video:", file);
        await invoke("play_test_video", { path: file });
      }
    } catch (e) {
      console.error("Failed to open video:", e);
    }
  };

  return (
    <div className="h-screen flex flex-col bg-background text-foreground">
      {/* Header */}
      <header className="border-b px-4 py-2 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h1 className="text-lg font-semibold">
            {project?.name || "TauriLivePlayer"}
          </h1>
          <Button variant="outline" size="sm" onClick={handleOpenVideo}>
            <FileVideo className="w-4 h-4 mr-2" />
            Open Video
          </Button>
        </div>
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <span className="capitalize">{status}</span>
          {error && <span className="text-destructive">{error}</span>}
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 flex overflow-hidden">
        {/* Left Panel - Cue List */}
        <div className="w-1/2 border-r flex flex-col">
          <div className="p-2 border-b bg-muted/50">
            <h2 className="text-sm font-medium">Cue List</h2>
          </div>
          <div className="flex-1 overflow-auto">
            <CueList />
          </div>
        </div>

        {/* Right Panel - Details & Controls */}
        <div className="w-1/2 flex flex-col">
          <div className="p-2 border-b bg-muted/50">
            <h2 className="text-sm font-medium">Brightness</h2>
          </div>
          <div className="p-4">
            <BrightnessPanel />
          </div>
        </div>
      </main>

      {/* Footer - Player Controls */}
      <footer className="border-t p-4">
        <PlayerControls />
      </footer>
    </div>
  );
}

export default App;
