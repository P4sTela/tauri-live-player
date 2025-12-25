import { Play, Pause, Square, SkipBack, SkipForward } from "lucide-react";
import { usePlayerStore } from "../../stores/playerStore";
import { Button } from "../ui/button";
import { Slider } from "../ui/slider";
import { formatTime } from "../../lib/utils";

export function PlayerControls() {
  const { status, currentTime, duration, play, pause, stop, seek, next, prev } =
    usePlayerStore();

  const isPlaying = status === "playing";
  const isReady =
    status !== "idle" && status !== "loading" && status !== "error";

  const handlePlayPause = () => {
    if (isPlaying) {
      pause();
    } else {
      play();
    }
  };

  const handleSeek = (value: number[]) => {
    seek(value[0]);
  };

  return (
    <div className="space-y-3">
      {/* Progress Bar */}
      <div className="flex items-center gap-3">
        <span className="text-sm text-muted-foreground w-14 text-right">
          {formatTime(currentTime)}
        </span>
        <Slider
          value={[currentTime]}
          min={0}
          max={duration || 100}
          step={0.1}
          onValueChange={handleSeek}
          disabled={!isReady}
          className="flex-1"
        />
        <span className="text-sm text-muted-foreground w-14">
          {formatTime(duration)}
        </span>
      </div>

      {/* Control Buttons */}
      <div className="flex items-center justify-center gap-2">
        <Button
          variant="outline"
          size="icon"
          onClick={prev}
          disabled={!isReady}
        >
          <SkipBack className="w-4 h-4" />
        </Button>

        <Button
          variant="outline"
          size="icon"
          onClick={stop}
          disabled={status === "idle"}
        >
          <Square className="w-4 h-4" />
        </Button>

        <Button
          variant="default"
          size="lg"
          className="w-16 h-12"
          onClick={handlePlayPause}
          disabled={!isReady}
        >
          {isPlaying ? (
            <Pause className="w-6 h-6" />
          ) : (
            <Play className="w-6 h-6" />
          )}
        </Button>

        <Button
          variant="outline"
          size="icon"
          onClick={next}
          disabled={!isReady}
        >
          <SkipForward className="w-4 h-4" />
        </Button>
      </div>
    </div>
  );
}
