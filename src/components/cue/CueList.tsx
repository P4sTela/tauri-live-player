import { Plus, Trash2, Play } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { usePlayerStore } from "../../stores/playerStore";
import { Button } from "../ui/button";
import { generateId } from "../../lib/utils";
import type { Cue } from "../../types";

export function CueList() {
  const { project, addCue, removeCue } = useProjectStore();
  const { currentCueIndex, loadCue, play } = usePlayerStore();

  const handleAddCue = () => {
    const newCue: Cue = {
      id: generateId(),
      name: `Cue ${(project?.cues.length || 0) + 1}`,
      items: [],
      duration: 0,
      loop: false,
      autoAdvance: false,
    };
    addCue(newCue);
  };

  const handleCueClick = async (index: number) => {
    await loadCue(index);
  };

  const handleCueDoubleClick = async (index: number) => {
    await loadCue(index);
    await play();
  };

  if (!project) {
    return <div className="p-4 text-muted-foreground">No project loaded</div>;
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1">
        {project.cues.length === 0 ? (
          <div className="p-4 text-center text-muted-foreground">
            No cues yet. Click "Add Cue" to create one.
          </div>
        ) : (
          <div className="divide-y">
            {project.cues.map((cue, index) => (
              <div
                key={cue.id}
                className={`flex items-center gap-2 px-4 py-2 cursor-pointer hover:bg-muted/50 transition-colors ${
                  index === currentCueIndex ? "bg-primary/10 border-l-2 border-primary" : ""
                }`}
                onClick={() => handleCueClick(index)}
                onDoubleClick={() => handleCueDoubleClick(index)}
              >
                <span className="w-8 text-muted-foreground text-sm">
                  {index + 1}
                </span>
                {index === currentCueIndex && (
                  <Play className="w-4 h-4 text-primary" />
                )}
                <span className="flex-1 truncate">{cue.name}</span>
                <span className="text-sm text-muted-foreground">
                  {cue.items.length} items
                </span>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 opacity-0 group-hover:opacity-100"
                  onClick={(e) => {
                    e.stopPropagation();
                    removeCue(cue.id);
                  }}
                >
                  <Trash2 className="w-4 h-4" />
                </Button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="p-2 border-t">
        <Button onClick={handleAddCue} variant="outline" className="w-full">
          <Plus className="w-4 h-4 mr-2" />
          Add Cue
        </Button>
      </div>
    </div>
  );
}
