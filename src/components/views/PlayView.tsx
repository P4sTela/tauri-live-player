import { Play, Pause, Square, SkipBack, SkipForward } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { usePlayerStore } from "../../stores/playerStore";
import { Button } from "../ui/button";
import { Slider } from "../ui/slider";
import { cn } from "../../lib/utils";

export function PlayView() {
  const { project } = useProjectStore();
  const {
    status,
    currentCueIndex,
    currentTime,
    duration,
    loadCue,
    play,
    pause,
    stop,
    seek,
    next,
    prev,
  } = usePlayerStore();

  const isPlaying = status === "playing";

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  const handleCueSelect = async (index: number) => {
    await loadCue(index);
  };

  const handleCuePlay = async (index: number) => {
    await loadCue(index);
    await play();
  };

  const handlePlayPause = async () => {
    if (isPlaying) {
      await pause();
    } else {
      await play();
    }
  };

  if (!project) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        No project loaded
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Cue List - Large, touch-friendly */}
      <div className="flex-1 overflow-auto">
        {project.cues.length === 0 ? (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            No cues. Switch to Edit tab to add cues.
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {project.cues.map((cue, index) => (
              <div
                key={cue.id}
                className={cn(
                  "flex items-center gap-3 p-4 rounded-lg cursor-pointer transition-all",
                  "hover:bg-muted/50",
                  index === currentCueIndex
                    ? "bg-primary/10 border-2 border-primary"
                    : "bg-muted/30 border-2 border-transparent"
                )}
                onClick={() => handleCueSelect(index)}
                onDoubleClick={() => handleCuePlay(index)}
              >
                {/* Cue Number */}
                <div
                  className={cn(
                    "w-10 h-10 rounded-full flex items-center justify-center text-lg font-bold",
                    index === currentCueIndex
                      ? "bg-primary text-primary-foreground"
                      : "bg-muted text-muted-foreground"
                  )}
                >
                  {index + 1}
                </div>

                {/* Cue Info */}
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-lg truncate">{cue.name}</div>
                  <div className="text-sm text-muted-foreground">
                    {cue.items.length} item{cue.items.length !== 1 ? "s" : ""}
                    {cue.loop && " • Loop"}
                    {cue.autoAdvance && " • Auto"}
                  </div>
                </div>

                {/* Play indicator */}
                {index === currentCueIndex && isPlaying && (
                  <div className="w-3 h-3 rounded-full bg-green-500 animate-pulse" />
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Transport Controls - Large, centered */}
      <div className="border-t bg-muted/30 p-6">
        {/* Progress Bar */}
        <div className="mb-4">
          <Slider
            value={[currentTime]}
            max={duration || 100}
            step={0.1}
            onValueChange={(v) => seek(v[0])}
            className="w-full"
          />
          <div className="flex justify-between text-sm text-muted-foreground mt-1">
            <span>{formatTime(currentTime)}</span>
            <span>{formatTime(duration)}</span>
          </div>
        </div>

        {/* Transport Buttons */}
        <div className="flex items-center justify-center gap-4">
          <Button
            variant="outline"
            size="lg"
            onClick={prev}
            disabled={currentCueIndex <= 0}
          >
            <SkipBack className="w-6 h-6" />
          </Button>

          <Button
            variant="outline"
            size="lg"
            onClick={stop}
          >
            <Square className="w-6 h-6" />
          </Button>

          <Button
            size="lg"
            className="w-20 h-20 rounded-full"
            onClick={handlePlayPause}
          >
            {isPlaying ? (
              <Pause className="w-10 h-10" />
            ) : (
              <Play className="w-10 h-10 ml-1" />
            )}
          </Button>

          <Button
            variant="outline"
            size="lg"
            onClick={next}
            disabled={currentCueIndex >= project.cues.length - 1}
          >
            <SkipForward className="w-6 h-6" />
          </Button>
        </div>

        {/* Current Cue Name */}
        {currentCueIndex >= 0 && project.cues[currentCueIndex] && (
          <div className="text-center mt-4 text-lg font-medium">
            {project.cues[currentCueIndex].name}
          </div>
        )}
      </div>
    </div>
  );
}
